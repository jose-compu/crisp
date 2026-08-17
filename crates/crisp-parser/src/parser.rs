use crisp_ast::{
    Span,
    expr::{
        BinaryOp, Block, Expr, ExprKind, FieldInit, MatchArm, Ownership, Param, Stmt, StringPart,
        StringParts, UnaryOp,
    },
    ident::Ident,
    item::{
        ConstDef, ExternBlock, ExternFn, FieldDef, FunctionDef, ImplBlock, Item, ShapeDef,
        ShapeField, SourceFile, TestDef, TraitDef, TraitItem, TypeBody, TypeDef, UseDecl,
        UseImport, VariantDef,
    },
    pat::{FieldPat, Pat, PatKind},
    ty::{ErrorType, Type, TypeBound, TypeKind},
};
use crisp_lexer::{Kw, Token, TokenKind, lex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("lex error: {0}")]
    Lex(#[from] crisp_lexer::LexError),
    #[error("{}", format_unexpected(.expected, .found, .pos, .help))]
    Unexpected {
        expected: &'static str,
        found: TokenKind,
        pos: u32,
        help: Option<&'static str>,
    },
    #[error("unexpected end of file, expected {expected}")]
    UnexpectedEof { expected: &'static str },
    #[error("invalid pattern at byte {pos}")]
    InvalidPat { pos: u32 },
}

fn format_unexpected(
    expected: &&'static str,
    found: &TokenKind,
    pos: &u32,
    help: &Option<&'static str>,
) -> String {
    let mut msg = format!("unexpected token {found:?} at byte {pos}, expected {expected}");
    if let Some(h) = help {
        msg.push_str("\nhelp: ");
        msg.push_str(h);
    }
    msg
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    /// When false, `Name { … }` is not parsed as a struct literal (Rust-style
    /// restriction for `if`/`while`/`for` conditions so `{` can start the body).
    allow_struct_lit: bool,
}

impl Parser {
    pub fn new(source: &str) -> Result<Self, ParseError> {
        Ok(Self {
            tokens: lex(source)?,
            pos: 0,
            allow_struct_lit: true,
        })
    }

    fn with_no_struct_lit<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<T, ParseError>,
    ) -> Result<T, ParseError> {
        let prev = self.allow_struct_lit;
        self.allow_struct_lit = false;
        let result = f(self);
        self.allow_struct_lit = prev;
        result
    }

    pub fn parse_file(&mut self) -> Result<SourceFile, ParseError> {
        let start = self.current_start();
        let mut items = Vec::new();
        while !self.check(TokenKind::Eof) {
            items.push(self.parse_item()?);
        }
        let end = if items.is_empty() {
            start
        } else {
            items.last().unwrap().span().end
        };
        Ok(SourceFile {
            items,
            span: Span::new(start, end),
        })
    }

    pub fn parse_module(&mut self) -> Result<Vec<Item>, ParseError> {
        Ok(self.parse_file()?.items)
    }

    // ── Items ─────────────────────────────────────────────────────────────

    fn parse_item(&mut self) -> Result<Item, ParseError> {
        let pub_span = if self.match_kw(Kw::Pub) {
            Some(self.previous_span())
        } else {
            None
        };
        let is_pub = pub_span.is_some();

        if self.match_kw(Kw::Type) {
            return Ok(Item::TypeDef(self.parse_type_def(is_pub)?));
        }
        if self.match_kw(Kw::Trait) {
            return Ok(Item::TraitDef(self.parse_trait_def()?));
        }
        if self.match_kw(Kw::Shape) {
            return Ok(Item::ShapeDef(self.parse_shape_def()?));
        }
        if self.match_kw(Kw::Impl) {
            return Ok(Item::Impl(self.parse_impl_block()?));
        }
        if self.match_kw(Kw::Use) {
            return Ok(Item::Use(self.parse_use_decl(is_pub)?));
        }
        if self.match_kw(Kw::Extern) {
            return Ok(Item::Extern(self.parse_extern_block()?));
        }
        if self.match_kw(Kw::TestCompileFail) {
            return Ok(Item::TestCompileFail(self.parse_test_def()?));
        }
        if self.match_kw(Kw::Test) {
            return Ok(Item::Test(self.parse_test_def()?));
        }

        let name = self.expect_ident()?;
        if self.check(TokenKind::Lt) || self.check(TokenKind::LParen) {
            return Ok(Item::Function(
                self.parse_function_after_name(is_pub, name)?,
            ));
        }
        if self.check(TokenKind::Assign) {
            self.advance();
            let value = self.parse_expr()?;
            let span = name.span.merge(value.span);
            return Ok(Item::Const(ConstDef { name, value, span }));
        }

        Err(self.unexpected("item", self.peek_kind()))
    }

    fn parse_function_after_name(
        &mut self,
        is_pub: bool,
        name: Ident,
    ) -> Result<FunctionDef, ParseError> {
        let start = name.span.start;
        let generics = self.parse_optional_generics()?;
        self.expect(TokenKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(TokenKind::RParen)?;
        let ret_type = if self.match_token(TokenKind::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };
        let error_type = if self.check(TokenKind::Bang) {
            Some(self.parse_error_type()?)
        } else {
            None
        };
        self.expect(TokenKind::Assign)?;
        let body = self.parse_expr()?;
        let end = body.span.end;
        Ok(FunctionDef {
            is_pub,
            name,
            generics,
            params,
            ret_type,
            error_type,
            body,
            span: Span::new(start, end),
        })
    }

    fn parse_params(&mut self) -> Result<Vec<Param>, ParseError> {
        let mut params = Vec::new();
        if self.check(TokenKind::RParen) {
            return Ok(params);
        }
        loop {
            params.push(self.parse_param()?);
            if !self.match_token(TokenKind::Comma) {
                break;
            }
        }
        Ok(params)
    }

    fn parse_param(&mut self) -> Result<Param, ParseError> {
        let start = self.current_start();
        let lifetime = if self.check_lifetime() {
            Some(self.parse_lifetime_ident()?)
        } else {
            None
        };
        let ownership = if self.match_kw(Kw::Own) {
            Some(Ownership::Own)
        } else if self.match_token(TokenKind::AmpMut) {
            Some(Ownership::RefMut)
        } else if self.match_token(TokenKind::Amp) {
            Some(Ownership::Ref)
        } else {
            None
        };
        let name = self.expect_ident()?;
        let ty = if self.match_token(TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        Ok(Param {
            lifetime,
            ownership,
            name,
            ty,
            span: Span::new(start, self.previous_end()),
        })
    }

    fn parse_type_def(&mut self, is_pub: bool) -> Result<TypeDef, ParseError> {
        let start = self.previous_start();
        let name = self.expect_ident()?;
        let generics = self.parse_optional_generics()?;
        self.expect(TokenKind::Assign)?;
        let (body, end) = if self.match_token(TokenKind::LBrace) {
            let fields = self.parse_struct_fields()?;
            self.expect(TokenKind::RBrace)?;
            let end = self.previous_end();
            (TypeBody::Struct(fields), end)
        } else if self.match_token(TokenKind::Pipe) {
            let variants = self.parse_enum_variants()?;
            (TypeBody::Enum(variants), self.previous_end())
        } else {
            let ty = self.parse_type()?;
            (TypeBody::Alias(ty.clone()), ty.span.end)
        };
        Ok(TypeDef {
            is_pub,
            name,
            generics,
            body,
            span: Span::new(start, end),
        })
    }

    fn parse_struct_fields(&mut self) -> Result<Vec<FieldDef>, ParseError> {
        let mut fields = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.check(TokenKind::Eof) {
            let start = self.current_start();
            let name = self.expect_ident()?;
            self.expect(TokenKind::Colon)?;
            let ty = self.parse_type()?;
            let default = if self.match_token(TokenKind::Assign) {
                Some(self.parse_expr()?)
            } else {
                None
            };
            fields.push(FieldDef {
                name,
                ty,
                default,
                span: Span::new(start, self.previous_end()),
            });
        }
        Ok(fields)
    }

    fn parse_enum_variants(&mut self) -> Result<Vec<VariantDef>, ParseError> {
        let mut variants = Vec::new();
        loop {
            let start = self.current_start();
            let name = self.expect_ident()?;
            let fields = if self.match_token(TokenKind::LParen) {
                let mut types = Vec::new();
                while !self.check(TokenKind::RParen) {
                    types.push(self.parse_type()?);
                    if !self.match_token(TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(TokenKind::RParen)?;
                types
            } else {
                vec![]
            };
            variants.push(VariantDef {
                name,
                fields,
                span: Span::new(start, self.previous_end()),
            });
            if !self.match_token(TokenKind::Pipe) {
                break;
            }
        }
        Ok(variants)
    }

    fn parse_trait_def(&mut self) -> Result<TraitDef, ParseError> {
        let start = self.previous_start();
        let name = self.expect_ident()?;
        let generics = self.parse_optional_generics()?;
        self.expect(TokenKind::Assign)?;
        self.expect(TokenKind::LBrace)?;
        let mut items = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.check(TokenKind::Eof) {
            items.push(self.parse_trait_item()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(TraitDef {
            name,
            generics,
            items,
            span: Span::new(start, self.previous_end()),
        })
    }

    fn parse_trait_item(&mut self) -> Result<TraitItem, ParseError> {
        let start = self.current_start();
        let name = self.expect_ident()?;
        self.expect(TokenKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(TokenKind::RParen)?;
        let ret_type = if self.match_token(TokenKind::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };
        let default_body = if self.match_token(TokenKind::Assign) {
            Some(self.parse_expr()?)
        } else {
            None
        };
        Ok(TraitItem {
            name,
            params,
            ret_type,
            default_body,
            span: Span::new(start, self.previous_end()),
        })
    }

    fn parse_shape_def(&mut self) -> Result<ShapeDef, ParseError> {
        let start = self.previous_start();
        let name = self.expect_ident()?;
        let generics = self.parse_optional_generics()?;
        self.expect(TokenKind::Assign)?;
        self.expect(TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.check(TokenKind::Eof) {
            fields.push(self.parse_shape_field()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(ShapeDef {
            name,
            generics,
            fields,
            span: Span::new(start, self.previous_end()),
        })
    }

    fn parse_shape_field(&mut self) -> Result<ShapeField, ParseError> {
        let start = self.current_start();
        let name = self.expect_ident()?;
        if self.match_token(TokenKind::Colon) {
            let ty = self.parse_type()?;
            return Ok(ShapeField::Data {
                name,
                ty,
                span: Span::new(start, self.previous_end()),
            });
        }
        self.expect(TokenKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(TokenKind::RParen)?;
        self.expect(TokenKind::Arrow)?;
        let ret_type = self.parse_type()?;
        Ok(ShapeField::Method {
            name,
            params,
            ret_type,
            span: Span::new(start, self.previous_end()),
        })
    }

    fn parse_impl_block(&mut self) -> Result<ImplBlock, ParseError> {
        let start = self.previous_start();
        let first = self.expect_ident()?;
        let trait_args = self.parse_optional_type_args()?;
        let (trait_name, ty) = if self.match_kw(Kw::For) {
            let ty = self.parse_type()?;
            (Some(first), ty)
        } else {
            (
                None,
                Type {
                    kind: TypeKind::Named(first.clone()),
                    span: first.span,
                },
            )
        };
        let mut items = Vec::new();
        if self.match_token(TokenKind::Assign) {
            self.expect(TokenKind::LBrace)?;
            while !self.check(TokenKind::RBrace) && !self.check(TokenKind::Eof) {
                let is_pub = self.match_kw(Kw::Pub);
                let name = self.expect_ident()?;
                items.push(self.parse_function_after_name(is_pub, name)?);
            }
            self.expect(TokenKind::RBrace)?;
        }
        Ok(ImplBlock {
            trait_name,
            trait_args,
            ty,
            items,
            span: Span::new(start, self.previous_end()),
        })
    }

    fn parse_use_decl(&mut self, is_pub: bool) -> Result<UseDecl, ParseError> {
        let start = self.previous_start();
        let mut path = vec![self.expect_ident()?];
        // Crisp module paths use `.`; spec §14.2 also writes `use rust::crate` — accept both.
        while self.match_token(TokenKind::Dot) || self.match_colon_colon() {
            path.push(self.expect_ident()?);
        }
        let imports = if self.match_token(TokenKind::LBrace) {
            let mut list = Vec::new();
            while !self.check(TokenKind::RBrace) {
                let s = self.current_start();
                let name = self.expect_ident()?;
                let alias = if self.match_kw(Kw::As) {
                    Some(self.expect_ident()?)
                } else {
                    None
                };
                list.push(UseImport {
                    name,
                    alias,
                    span: Span::new(s, self.previous_end()),
                });
                if !self.match_token(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RBrace)?;
            Some(list)
        } else {
            None
        };
        Ok(UseDecl {
            is_pub,
            path,
            imports,
            span: Span::new(start, self.previous_end()),
        })
    }

    fn parse_extern_block(&mut self) -> Result<ExternBlock, ParseError> {
        let start = self.previous_start();
        let abi = self.expect_string_lit()?;
        self.expect(TokenKind::LBrace)?;
        let mut functions = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.check(TokenKind::Eof) {
            let s = self.current_start();
            let name = self.expect_ident()?;
            self.expect(TokenKind::LParen)?;
            let params = self.parse_params()?;
            self.expect(TokenKind::RParen)?;
            let ret_type = if self.match_token(TokenKind::Arrow) {
                Some(self.parse_type()?)
            } else {
                None
            };
            functions.push(ExternFn {
                name,
                params,
                ret_type,
                span: Span::new(s, self.previous_end()),
            });
        }
        self.expect(TokenKind::RBrace)?;
        Ok(ExternBlock {
            abi,
            functions,
            span: Span::new(start, self.previous_end()),
        })
    }

    fn parse_test_def(&mut self) -> Result<TestDef, ParseError> {
        let start = self.previous_start();
        let name = self.expect_string_lit()?;
        self.expect(TokenKind::Assign)?;
        let body = self.parse_block()?;
        let end = body.span.end;
        Ok(TestDef {
            name,
            body,
            span: Span::new(start, end),
        })
    }

    // ── Types ─────────────────────────────────────────────────────────────

    fn parse_type(&mut self) -> Result<Type, ParseError> {
        let start = self.current_start();
        if self.match_token(TokenKind::Question) {
            let inner = self.parse_type()?;
            return Ok(Type {
                kind: TypeKind::Option(Box::new(inner)),
                span: Span::new(start, self.previous_end()),
            });
        }
        if self.match_token(TokenKind::AmpMut) {
            let inner = self.parse_type()?;
            return Ok(Type {
                kind: TypeKind::Ref {
                    mutable: true,
                    inner: Box::new(inner),
                },
                span: Span::new(start, self.previous_end()),
            });
        }
        if self.match_token(TokenKind::Amp) {
            let inner = self.parse_type()?;
            return Ok(Type {
                kind: TypeKind::Ref {
                    mutable: false,
                    inner: Box::new(inner),
                },
                span: Span::new(start, self.previous_end()),
            });
        }
        if self.match_token(TokenKind::LParen) {
            if self.check(TokenKind::RParen) {
                self.advance();
                return Ok(Type {
                    kind: TypeKind::Unit,
                    span: Span::new(start, self.previous_end()),
                });
            }
            let mut types = vec![self.parse_type()?];
            while self.match_token(TokenKind::Comma) {
                types.push(self.parse_type()?);
            }
            self.expect(TokenKind::RParen)?;
            if types.len() == 1 && self.check(TokenKind::Arrow) {
                self.advance();
                let ret = self.parse_type()?;
                return Ok(Type {
                    kind: TypeKind::Fn {
                        params: types,
                        ret: Box::new(ret),
                    },
                    span: Span::new(start, self.previous_end()),
                });
            }
            return Ok(Type {
                kind: TypeKind::Tuple(types),
                span: Span::new(start, self.previous_end()),
            });
        }
        if self.match_token(TokenKind::LBracket) {
            let elem = self.parse_type()?;
            if self.match_token(TokenKind::Semi) && self.is_int() {
                let TokenKind::Int(n) = self.advance().kind else {
                    unreachable!();
                };
                self.expect(TokenKind::RBracket)?;
                return Ok(Type {
                    kind: TypeKind::Array {
                        elem: Box::new(elem),
                        len: n as u64,
                    },
                    span: Span::new(start, self.previous_end()),
                });
            }
            self.expect(TokenKind::RBracket)?;
            return Ok(Type {
                kind: TypeKind::Slice(Box::new(elem)),
                span: Span::new(start, self.previous_end()),
            });
        }

        let base = self.parse_type_primary()?;
        let mut ty = base;
        if self.match_token(TokenKind::Lt) {
            let mut args = vec![self.parse_type()?];
            while self.match_token(TokenKind::Comma) {
                args.push(self.parse_type()?);
            }
            self.expect(TokenKind::Gt)?;
            ty = Type {
                kind: TypeKind::Generic {
                    base: Box::new(ty),
                    args,
                },
                span: Span::new(start, self.previous_end()),
            };
        }
        while self.match_token(TokenKind::Plus) {
            let mut bounds = vec![self.parse_type_bound()?];
            while self.match_token(TokenKind::Plus) {
                bounds.push(self.parse_type_bound()?);
            }
            ty = Type {
                kind: TypeKind::Constrained {
                    inner: Box::new(ty),
                    bounds,
                },
                span: Span::new(start, self.previous_end()),
            };
        }
        Ok(ty)
    }

    fn parse_type_primary(&mut self) -> Result<Type, ParseError> {
        let start = self.current_start();
        if let Some(kw) = self.match_kw_opt() {
            let name = match kw {
                Kw::True | Kw::False => "bool",
                _ => return Err(self.unexpected("type", TokenKind::Kw(kw))),
            };
            return Ok(Type {
                kind: TypeKind::Named(Ident::new(name, Span::new(start, self.previous_end()))),
                span: Span::new(start, self.previous_end()),
            });
        }
        let name = self.expect_ident()?;
        let kind = match name.name.as_str() {
            "Never" => TypeKind::Never,
            "()" => TypeKind::Unit,
            _ => TypeKind::Named(name.clone()),
        };
        Ok(Type {
            kind,
            span: name.span,
        })
    }

    fn parse_type_bound(&mut self) -> Result<TypeBound, ParseError> {
        if self.match_kw(Kw::Shape) {
            return Ok(TypeBound::Shape(self.expect_ident()?));
        }
        Ok(TypeBound::Trait(self.expect_ident()?))
    }

    fn parse_error_type(&mut self) -> Result<ErrorType, ParseError> {
        let start = self.current_start();
        self.expect(TokenKind::Bang)?;
        if self.match_kw(Kw::False) {
            // !never — use Never keyword path; accept `!never` as ident
        }
        let first = self.parse_type()?;
        let mut variants = vec![first];
        while self.match_token(TokenKind::Pipe) {
            variants.push(self.parse_type()?);
        }
        Ok(ErrorType {
            variants,
            span: Span::new(start, self.previous_end()),
        })
    }

    fn parse_optional_generics(&mut self) -> Result<Vec<Ident>, ParseError> {
        if !self.match_token(TokenKind::Lt) {
            return Ok(vec![]);
        }
        let mut names = vec![self.expect_ident()?];
        while self.match_token(TokenKind::Comma) {
            names.push(self.expect_ident()?);
        }
        self.expect(TokenKind::Gt)?;
        Ok(names)
    }

    fn parse_optional_type_args(&mut self) -> Result<Vec<Type>, ParseError> {
        if !self.match_token(TokenKind::Lt) {
            return Ok(vec![]);
        }
        let mut args = vec![self.parse_type()?];
        while self.match_token(TokenKind::Comma) {
            args.push(self.parse_type()?);
        }
        self.expect(TokenKind::Gt)?;
        Ok(args)
    }

    // ── Expressions ───────────────────────────────────────────────────────

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_pipe()
    }

    fn parse_pipe(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_assign()?;
        while self.match_token(TokenKind::PipeGt) {
            let right = self.parse_assign()?;
            let span = left.span.merge(right.span);
            left = Expr {
                kind: ExprKind::Pipe {
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            };
        }
        left = self.parse_catch_suffix(left)?;
        Ok(left)
    }

    fn parse_catch_suffix(&mut self, expr: Expr) -> Result<Expr, ParseError> {
        let mut arms = Vec::new();
        while self.match_kw(Kw::Catch) {
            let start = self.previous_start();
            let pat = self.parse_pat()?;
            self.expect(TokenKind::Arrow)?;
            let body = self.parse_pipe()?;
            let body_end = body.span.end;
            arms.push(crisp_ast::expr::CatchArm {
                pat,
                body,
                span: Span::new(start, body_end),
            });
        }
        if arms.is_empty() {
            return Ok(expr);
        }
        let expr_start = expr.span.start;
        let end = arms.last().map(|a| a.span.end).unwrap_or(expr.span.end);
        Ok(Expr {
            kind: ExprKind::Catch {
                body: Box::new(expr),
                arms,
            },
            span: Span::new(expr_start, end),
        })
    }

    fn parse_assign(&mut self) -> Result<Expr, ParseError> {
        let expr = self.parse_or()?;
        if self.match_token(TokenKind::Assign) {
            let (name, id_span) = match &expr.kind {
                ExprKind::Ident(id) => (id.name.clone(), id.span),
                _ => return Err(self.unexpected("assignable identifier", self.peek_kind())),
            };
            let value = self.parse_assign()?;
            let span = id_span.merge(value.span);
            return Ok(Expr {
                kind: ExprKind::Assign {
                    target: Ident::new(name, id_span),
                    value: Box::new(value),
                },
                span,
            });
        }
        Ok(expr)
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        self.parse_binary(Self::parse_and, TokenKind::Or, BinaryOp::Or)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        self.parse_binary(Self::parse_equality, TokenKind::And, BinaryOp::And)
    }

    fn parse_equality(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_comparison()?;
        while matches!(self.peek_kind(), TokenKind::EqEq | TokenKind::Ne) {
            let op = match self.advance().kind {
                TokenKind::EqEq => BinaryOp::Eq,
                TokenKind::Ne => BinaryOp::Ne,
                _ => unreachable!(),
            };
            let right = self.parse_comparison()?;
            let span = left.span.merge(right.span);
            left = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_concat()?;
        while matches!(
            self.peek_kind(),
            TokenKind::Lt | TokenKind::Le | TokenKind::Gt | TokenKind::Ge
        ) {
            let op = match self.advance().kind {
                TokenKind::Lt => BinaryOp::Lt,
                TokenKind::Le => BinaryOp::Le,
                TokenKind::Gt => BinaryOp::Gt,
                TokenKind::Ge => BinaryOp::Ge,
                _ => unreachable!(),
            };
            let right = self.parse_concat()?;
            let span = left.span.merge(right.span);
            left = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(left)
    }

    fn parse_concat(&mut self) -> Result<Expr, ParseError> {
        self.parse_binary(Self::parse_additive, TokenKind::PlusPlus, BinaryOp::Concat)
    }

    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            let span = left.span.merge(right.span);
            left = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_power()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                TokenKind::Percent => BinaryOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_power()?;
            let span = left.span.merge(right.span);
            left = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(left)
    }

    fn parse_power(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_unary()?;
        if self.match_token(TokenKind::StarStar) {
            let right = self.parse_power()?;
            let span = left.span.merge(right.span);
            left = Expr {
                kind: ExprKind::Binary {
                    op: BinaryOp::Pow,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        let start = self.current_start();
        if self.match_token(TokenKind::Bang) {
            let expr = self.parse_unary()?;
            let end = expr.span.end;
            return Ok(Expr {
                kind: ExprKind::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                },
                span: Span::new(start, end),
            });
        }
        if self.match_token(TokenKind::Minus) {
            let expr = self.parse_unary()?;
            let end = expr.span.end;
            return Ok(Expr {
                kind: ExprKind::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(expr),
                },
                span: Span::new(start, end),
            });
        }
        if self.match_kw(Kw::Async) {
            let body = self.parse_unary()?;
            let end = body.span.end;
            return Ok(Expr {
                kind: ExprKind::Async(Box::new(body)),
                span: Span::new(start, end),
            });
        }
        if self.match_kw(Kw::Await) {
            let body = self.parse_unary()?;
            let end = body.span.end;
            return Ok(Expr {
                kind: ExprKind::Await(Box::new(body)),
                span: Span::new(start, end),
            });
        }
        if self.match_kw(Kw::Spawn) {
            let body = self.parse_unary()?;
            let end = body.span.end;
            return Ok(Expr {
                kind: ExprKind::Spawn(Box::new(body)),
                span: Span::new(start, end),
            });
        }
        if self.match_kw(Kw::Unsafe) {
            let body = self.parse_unary()?;
            let end = body.span.end;
            return Ok(Expr {
                kind: ExprKind::Unsafe(Box::new(body)),
                span: Span::new(start, end),
            });
        }
        if self.match_kw(Kw::Return) {
            let value = if self.check_expr_start() {
                Some(Box::new(self.parse_expr()?))
            } else {
                None
            };
            let end = value
                .as_ref()
                .map(|v| v.span.end)
                .unwrap_or(self.previous_end());
            return Ok(Expr {
                kind: ExprKind::Return(value),
                span: Span::new(start, end),
            });
        }
        if self.match_kw(Kw::Break) {
            let value = if self.check_expr_start() {
                Some(Box::new(self.parse_expr()?))
            } else {
                None
            };
            let end = value
                .as_ref()
                .map(|v| v.span.end)
                .unwrap_or(self.previous_end());
            return Ok(Expr {
                kind: ExprKind::Break(value),
                span: Span::new(start, end),
            });
        }
        if self.match_kw(Kw::Continue) {
            return Ok(Expr {
                kind: ExprKind::Continue,
                span: Span::new(start, self.previous_end()),
            });
        }
        if self.match_kw(Kw::Throw) {
            let expr = self.parse_unary()?;
            let end = expr.span.end;
            return Ok(Expr {
                kind: ExprKind::Throw(Box::new(expr)),
                span: Span::new(start, end),
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.peek_kind() {
                TokenKind::LParen => {
                    self.advance();
                    let args = self.parse_args()?;
                    self.expect(TokenKind::RParen)?;
                    let span = expr
                        .span
                        .merge(Span::new(self.previous_start(), self.previous_end()));
                    expr = Expr {
                        kind: ExprKind::Call {
                            func: Box::new(expr),
                            args,
                        },
                        span,
                    };
                }
                TokenKind::LBrace if self.peek_kind_at(1) == TokenKind::Pipe => {
                    let start = self.current_start();
                    self.advance();
                    let lam = self.parse_lambda(start)?;
                    self.expect(TokenKind::RBrace)?;
                    let span = expr.span.merge(lam.span);
                    expr = match expr.kind {
                        ExprKind::Call { func, mut args } => {
                            args.push(lam);
                            Expr {
                                kind: ExprKind::Call { func, args },
                                span,
                            }
                        }
                        kind => Expr {
                            kind: ExprKind::Call {
                                func: Box::new(Expr {
                                    kind,
                                    span: expr.span,
                                }),
                                args: vec![lam],
                            },
                            span,
                        },
                    };
                }
                TokenKind::Dot => {
                    self.advance();
                    let field = self.expect_ident()?;
                    let span = expr.span.merge(field.span);
                    expr = Expr {
                        kind: ExprKind::Field {
                            base: Box::new(expr),
                            field,
                        },
                        span,
                    };
                }
                TokenKind::LBracket => {
                    self.advance();
                    let index = self.parse_expr()?;
                    self.expect(TokenKind::RBracket)?;
                    let span = expr.span.merge(index.span);
                    expr = Expr {
                        kind: ExprKind::Index {
                            base: Box::new(expr),
                            index: Box::new(index),
                        },
                        span,
                    };
                }
                TokenKind::Question => {
                    self.advance();
                    let span = expr
                        .span
                        .merge(Span::new(self.previous_start(), self.previous_end()));
                    expr = Expr {
                        kind: ExprKind::Try(Box::new(expr)),
                        span,
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let start = self.current_start();
        match self.peek_kind() {
            TokenKind::Int(n) => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Int(n),
                    span: Span::new(start, self.previous_end()),
                })
            }
            TokenKind::Float(f) => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Float(f),
                    span: Span::new(start, self.previous_end()),
                })
            }
            TokenKind::String(ref s) => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Str(self.parse_string_parts(s, start)?),
                    span: Span::new(start, self.previous_end()),
                })
            }
            TokenKind::Char(c) => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Char(c),
                    span: Span::new(start, self.previous_end()),
                })
            }
            TokenKind::Kw(Kw::True) => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Bool(true),
                    span: Span::new(start, self.previous_end()),
                })
            }
            TokenKind::Kw(Kw::False) => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Bool(false),
                    span: Span::new(start, self.previous_end()),
                })
            }
            TokenKind::Kw(Kw::None) => {
                self.advance();
                Ok(self.parse_none_some_call(false, start)?)
            }
            TokenKind::Kw(Kw::Some) => {
                self.advance();
                Ok(self.parse_none_some_call(true, start)?)
            }
            TokenKind::Ident(_) | TokenKind::Kw(_) => {
                if self.match_kw(Kw::If) {
                    return self.parse_if_expr(start);
                }
                if self.match_kw(Kw::Match) {
                    return self.parse_match_expr(start);
                }
                if self.match_kw(Kw::For) {
                    return self.parse_for_expr(start);
                }
                if self.match_kw(Kw::While) {
                    return self.parse_while_expr(start);
                }
                if self.match_kw(Kw::Loop) {
                    return self.parse_loop_expr(start);
                }
                let id = self.expect_ident_or_kw_as_ident()?;
                let span = id.span;
                // Trailing last-arg lambda: `run { |x| … }` — not a struct literal (#88).
                if self.check(TokenKind::LBrace) && self.peek_kind_at(1) == TokenKind::Pipe {
                    return Ok(Expr {
                        kind: ExprKind::Ident(id),
                        span,
                    });
                }
                // struct literal: Name { ... } (disabled in if/while/for heads)
                if self.allow_struct_lit && self.check(TokenKind::LBrace) {
                    return self.parse_struct_lit(id);
                }
                Ok(Expr {
                    kind: ExprKind::Ident(id),
                    span,
                })
            }
            TokenKind::LBrace => Ok(Expr {
                kind: ExprKind::Block(self.parse_block()?),
                span: Span::new(start, self.previous_end()),
            }),
            TokenKind::Dot => self.parse_point_free_section(start),
            TokenKind::LParen => {
                self.advance();
                if self.check(TokenKind::RParen) {
                    self.advance();
                    return Ok(Expr {
                        kind: ExprKind::Unit,
                        span: Span::new(start, self.previous_end()),
                    });
                }
                if self.check(TokenKind::Pipe)
                    || (self.is_ident() && self.peek_kind_at(1) == TokenKind::Pipe)
                {
                    let lam = self.parse_lambda(start)?;
                    self.expect(TokenKind::RParen)?;
                    return Ok(lam);
                }
                let expr = self.parse_expr()?;
                self.expect(TokenKind::RParen)?;
                Ok(expr)
            }
            TokenKind::Pipe => self.parse_lambda(start),
            TokenKind::Or => {
                // `|| expr` lexes as one `Or` token; treat as a nullary lambda.
                self.advance();
                let body = self.parse_expr()?;
                let end = body.span.end;
                Ok(Expr {
                    kind: ExprKind::Lambda {
                        params: Vec::new(),
                        body: Box::new(body),
                    },
                    span: Span::new(start, end),
                })
            }
            _ => Err(self.unexpected("expression", self.peek_kind())),
        }
    }

    fn parse_none_some_call(&mut self, some: bool, start: u32) -> Result<Expr, ParseError> {
        if self.match_token(TokenKind::LParen) {
            let inner = self.parse_expr()?;
            self.expect(TokenKind::RParen)?;
            let name = if some { "some" } else { "none" };
            return Ok(Expr {
                kind: ExprKind::Call {
                    func: Box::new(Expr {
                        kind: ExprKind::Ident(Ident::new(name, Span::new(start, start))),
                        span: Span::new(start, start),
                    }),
                    args: if some { vec![inner] } else { vec![] },
                },
                span: Span::new(start, self.previous_end()),
            });
        }
        let name = if some { "some" } else { "none" };
        Ok(Expr {
            kind: ExprKind::Ident(Ident::new(name, Span::new(start, self.previous_end()))),
            span: Span::new(start, self.previous_end()),
        })
    }

    fn parse_lambda(&mut self, start: u32) -> Result<Expr, ParseError> {
        self.expect(TokenKind::Pipe)?;
        let params = if self.check(TokenKind::Pipe) {
            Vec::new()
        } else {
            self.parse_params()?
        };
        self.expect(TokenKind::Pipe)?;
        let body = self.parse_expr()?;
        let end = body.span.end;
        Ok(Expr {
            kind: ExprKind::Lambda {
                params,
                body: Box::new(body),
            },
            span: Span::new(start, end),
        })
    }

    fn parse_if_expr(&mut self, start: u32) -> Result<Expr, ParseError> {
        let cond = self.with_no_struct_lit(|p| p.parse_expr())?;
        let then_branch = if self.match_kw(Kw::Then) {
            Box::new(self.parse_expr()?)
        } else {
            Box::new(Expr {
                kind: ExprKind::Block(self.parse_block()?),
                span: Span::new(self.current_start(), self.previous_end()),
            })
        };
        let else_branch = if self.match_kw(Kw::Else) {
            if self.check(TokenKind::Kw(Kw::If)) {
                Some(Box::new(self.parse_if_expr(self.current_start())?))
            } else if self.check(TokenKind::LBrace) {
                Some(Box::new(Expr {
                    kind: ExprKind::Block(self.parse_block()?),
                    span: Span::new(self.current_start(), self.previous_end()),
                }))
            } else {
                Some(Box::new(self.parse_expr()?))
            }
        } else {
            None
        };
        let end = else_branch
            .as_ref()
            .map(|e| e.span.end)
            .unwrap_or(then_branch.span.end);
        Ok(Expr {
            kind: ExprKind::If {
                cond: Box::new(cond),
                then_branch,
                else_branch,
            },
            span: Span::new(start, end),
        })
    }

    fn parse_match_expr(&mut self, start: u32) -> Result<Expr, ParseError> {
        let scrutinee = self.parse_match_scrutinee()?;
        self.expect(TokenKind::LBrace)?;
        let mut arms = Vec::new();
        while !self.check(TokenKind::RBrace) {
            arms.push(self.parse_match_arm()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Expr {
            kind: ExprKind::Match {
                scrutinee: Box::new(scrutinee),
                arms,
            },
            span: Span::new(start, self.previous_end()),
        })
    }

    /// Scrutinee parsing that does not treat `name {` as a struct literal.
    /// `match color { … }` would otherwise consume `color { … }` as `StructLit`.
    fn parse_match_scrutinee(&mut self) -> Result<Expr, ParseError> {
        if matches!(self.peek_kind(), TokenKind::Ident(_))
            && matches!(self.peek_kind_at(1), TokenKind::LBrace)
        {
            let id = self.expect_ident()?;
            return Ok(Expr {
                kind: ExprKind::Ident(id.clone()),
                span: id.span,
            });
        }
        self.parse_expr()
    }

    fn parse_match_arm(&mut self) -> Result<MatchArm, ParseError> {
        let start = self.current_start();
        let pat = self.parse_pat()?;
        let guard = if self.match_kw(Kw::If) {
            Some(self.parse_expr()?)
        } else {
            None
        };
        if !self.match_token(TokenKind::Arrow) {
            return Err(ParseError::Unexpected {
                expected: "`->`",
                found: self.peek_kind(),
                pos: self.current_start(),
                help: Some(
                    "if the match scrutinee is a struct literal, wrap it in parentheses: \
match (Name { field: value }) { ... }",
                ),
            });
        }
        let body = self.parse_expr()?;
        let end = body.span.end;
        Ok(MatchArm {
            pat,
            guard,
            body,
            span: Span::new(start, end),
        })
    }

    fn parse_for_expr(&mut self, start: u32) -> Result<Expr, ParseError> {
        let pat = self.parse_pat()?;
        self.expect_kw(Kw::In)?;
        let iter = self.with_no_struct_lit(|p| p.parse_expr())?;
        let body = Expr {
            kind: ExprKind::Block(self.parse_block()?),
            span: Span::new(self.current_start(), self.previous_end()),
        };
        let end = body.span.end;
        Ok(Expr {
            kind: ExprKind::For {
                pat,
                iter: Box::new(iter),
                body: Box::new(body),
            },
            span: Span::new(start, end),
        })
    }

    fn parse_while_expr(&mut self, start: u32) -> Result<Expr, ParseError> {
        let cond = self.with_no_struct_lit(|p| p.parse_expr())?;
        let body = Expr {
            kind: ExprKind::Block(self.parse_block()?),
            span: Span::new(self.current_start(), self.previous_end()),
        };
        let end = body.span.end;
        Ok(Expr {
            kind: ExprKind::While {
                cond: Box::new(cond),
                body: Box::new(body),
            },
            span: Span::new(start, end),
        })
    }

    fn parse_loop_expr(&mut self, start: u32) -> Result<Expr, ParseError> {
        let body = Expr {
            kind: ExprKind::Block(self.parse_block()?),
            span: Span::new(self.current_start(), self.previous_end()),
        };
        let end = body.span.end;
        Ok(Expr {
            kind: ExprKind::Loop(Box::new(body)),
            span: Span::new(start, end),
        })
    }

    fn parse_struct_lit(&mut self, name: Ident) -> Result<Expr, ParseError> {
        let start = name.span.start;
        self.expect(TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while !self.check(TokenKind::RBrace) {
            let s = self.current_start();
            let fname = self.expect_ident()?;
            self.expect(TokenKind::Colon)?;
            let value = self.parse_expr()?;
            fields.push(FieldInit {
                name: fname,
                value,
                span: Span::new(s, self.previous_end()),
            });
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Expr {
            kind: ExprKind::StructLit { name, fields },
            span: Span::new(start, self.previous_end()),
        })
    }

    fn parse_block(&mut self) -> Result<Block, ParseError> {
        let start = self.current_start();
        self.expect(TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        let mut tail = None;
        while !self.check(TokenKind::RBrace) && !self.check(TokenKind::Eof) {
            let saved = self.pos;
            if let Ok(stmt) = self.try_parse_binding_stmt() {
                stmts.push(stmt);
                continue;
            }
            self.pos = saved;

            if self.is_ident() && self.peek_kind_at(1) == TokenKind::Assign {
                let target = self.expect_ident()?;
                self.advance();
                let value = self.parse_expr()?;
                stmts.push(Stmt::Assign { target, value });
                continue;
            }

            let expr = self.parse_expr()?;
            if self.check(TokenKind::RBrace) {
                tail = Some(Box::new(expr));
                break;
            }
            stmts.push(Stmt::Expr(expr));
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Block {
            stmts,
            tail,
            span: Span::new(start, self.previous_end()),
        })
    }

    fn try_parse_binding_stmt(&mut self) -> Result<Stmt, ParseError> {
        let pat = self.parse_pat()?;
        let mutable = if self.match_token(TokenKind::MutColonEq) {
            true
        } else if self.match_token(TokenKind::ColonEq) {
            false
        } else {
            return Err(ParseError::Unexpected {
                expected: "`:=` or `mut:`=",
                found: self.peek_kind(),
                pos: self.current_start(),
                help: None,
            });
        };
        let value = self.parse_expr()?;
        Ok(Stmt::Bind {
            pat,
            mutable,
            value,
        })
    }

    // ── Patterns ──────────────────────────────────────────────────────────

    fn parse_pat(&mut self) -> Result<Pat, ParseError> {
        let start = self.current_start();
        if let TokenKind::Ident(name) = self.peek_kind()
            && name == "_"
        {
            self.advance();
            return Ok(Pat {
                kind: PatKind::Wildcard,
                span: Span::new(start, self.previous_end()),
            });
        }
        if self.is_int() || self.is_string() {
            let expr = self.parse_primary()?;
            return Ok(Pat {
                kind: PatKind::Literal(Box::new(expr)),
                span: Span::new(start, self.previous_end()),
            });
        }
        if self.match_token(TokenKind::LParen) {
            if self.check(TokenKind::RParen) {
                self.advance();
                return Ok(Pat {
                    kind: PatKind::Literal(Box::new(Expr {
                        kind: ExprKind::Unit,
                        span: Span::new(start, self.previous_end()),
                    })),
                    span: Span::new(start, self.previous_end()),
                });
            }
            let mut pats = vec![self.parse_pat()?];
            while self.match_token(TokenKind::Comma) {
                pats.push(self.parse_pat()?);
            }
            self.expect(TokenKind::RParen)?;
            return Ok(Pat {
                kind: PatKind::Tuple(pats),
                span: Span::new(start, self.previous_end()),
            });
        }
        if self.match_token(TokenKind::LBrace) {
            let name = self.expect_ident()?;
            let mut fields = Vec::new();
            let mut rest = None;
            while !self.check(TokenKind::RBrace) {
                if self.match_token(TokenKind::DotDot) {
                    rest = Some(self.expect_ident()?);
                    break;
                }
                let fname = self.expect_ident()?;
                let pat = if self.match_token(TokenKind::Colon) {
                    Some(self.parse_pat()?)
                } else {
                    None
                };
                fields.push(FieldPat {
                    name: fname,
                    pat,
                    span: Span::new(start, self.previous_end()),
                });
            }
            self.expect(TokenKind::RBrace)?;
            return Ok(Pat {
                kind: PatKind::Struct { name, fields, rest },
                span: Span::new(start, self.previous_end()),
            });
        }
        let name = self.expect_ident()?;
        // Qualified enum pattern: Color.Red / Color.Custom(r, g, b)
        if self.match_token(TokenKind::Dot) {
            let variant = self.expect_ident()?;
            let args = if self.match_token(TokenKind::LParen) {
                let mut args = Vec::new();
                while !self.check(TokenKind::RParen) {
                    args.push(self.parse_pat()?);
                    if !self.match_token(TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(TokenKind::RParen)?;
                args
            } else {
                vec![]
            };
            return Ok(Pat {
                kind: PatKind::Enum {
                    name,
                    variant,
                    args,
                },
                span: Span::new(start, self.previous_end()),
            });
        }
        // Unqualified ctor pattern: Custom(r, g, b) — variant name equals type slot
        if self.match_token(TokenKind::LParen) {
            let mut args = Vec::new();
            while !self.check(TokenKind::RParen) {
                args.push(self.parse_pat()?);
                if !self.match_token(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RParen)?;
            return Ok(Pat {
                kind: PatKind::Enum {
                    name: name.clone(),
                    variant: name,
                    args,
                },
                span: Span::new(start, self.previous_end()),
            });
        }
        Ok(Pat {
            kind: PatKind::Ident(name),
            span: Span::new(start, self.previous_end()),
        })
    }

    // ── Helpers ───────────────────────────────────────────────────────────

    fn parse_binary<F>(
        &mut self,
        mut next: F,
        tok: TokenKind,
        op: BinaryOp,
    ) -> Result<Expr, ParseError>
    where
        F: FnMut(&mut Self) -> Result<Expr, ParseError>,
    {
        let mut left = next(self)?;
        while self.check(tok.clone()) {
            self.advance();
            let right = next(self)?;
            let span = left.span.merge(right.span);
            left = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(left)
    }

    #[allow(clippy::while_let_on_iterator)]
    fn parse_string_parts(&mut self, s: &str, start: u32) -> Result<StringParts, ParseError> {
        let mut parts = Vec::new();
        let mut lit = String::new();
        let mut chars = s.chars().peekable();
        let iter = chars.by_ref();
        while let Some(c) = iter.next() {
            if c == '{' {
                if !lit.is_empty() {
                    parts.push(StringPart::Lit(std::mem::take(&mut lit)));
                }
                let mut depth = 1i32;
                let mut expr_text = String::new();
                while let Some(ch) = iter.next() {
                    if ch == '{' {
                        depth += 1;
                    }
                    if ch == '}' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    expr_text.push(ch);
                }
                let mut sub = Parser::new(&expr_text)?;
                let expr = sub.parse_expr()?;
                parts.push(StringPart::Expr(Box::new(expr)));
            } else {
                lit.push(c);
            }
        }
        if !lit.is_empty() {
            parts.push(StringPart::Lit(lit));
        }
        if parts.is_empty() {
            parts.push(StringPart::Lit(String::new()));
        }
        let _ = start;
        Ok(StringParts(parts))
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut args = Vec::new();
        if self.check(TokenKind::RParen) {
            return Ok(args);
        }
        loop {
            args.push(self.parse_expr()?);
            if !self.match_token(TokenKind::Comma) {
                break;
            }
        }
        Ok(args)
    }

    fn is_ident(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Ident(_))
    }

    fn is_int(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Int(_))
    }

    fn is_string(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::String(_))
    }

    fn check_expr_start(&self) -> bool {
        !matches!(
            self.peek_kind(),
            TokenKind::RBrace | TokenKind::Eof | TokenKind::RParen
        )
    }

    fn check_lifetime(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Lifetime(_))
    }

    fn parse_lifetime_ident(&mut self) -> Result<Ident, ParseError> {
        let t = self.advance();
        let TokenKind::Lifetime(name) = t.kind else {
            return Err(self.unexpected("lifetime", t.kind));
        };
        Ok(Ident::new(name, Span::new(t.start, t.end)))
    }

    fn expect_ident(&mut self) -> Result<Ident, ParseError> {
        let t = self.advance();
        match t.kind {
            TokenKind::Ident(name) => Ok(Ident::new(name, Span::new(t.start, t.end))),
            other => Err(ParseError::Unexpected {
                expected: "identifier",
                found: other,
                pos: t.start,
                help: None,
            }),
        }
    }

    fn expect_ident_or_kw_as_ident(&mut self) -> Result<Ident, ParseError> {
        self.expect_ident()
    }

    fn expect_string_lit(&mut self) -> Result<String, ParseError> {
        let t = self.advance();
        match t.kind {
            TokenKind::String(s) => Ok(s),
            other => Err(ParseError::Unexpected {
                expected: "string literal",
                found: other,
                pos: t.start,
                help: None,
            }),
        }
    }

    fn expect_kw(&mut self, kw: Kw) -> Result<(), ParseError> {
        if self.match_kw(kw) {
            Ok(())
        } else {
            Err(self.unexpected(
                std::str::from_utf8(kw_bytes(kw)).unwrap_or("keyword"),
                self.peek_kind(),
            ))
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Result<(), ParseError> {
        if self.match_token(kind.clone()) {
            Ok(())
        } else {
            Err(self.unexpected("token", self.peek_kind()))
        }
    }

    fn match_kw(&mut self, kw: Kw) -> bool {
        if matches!(self.peek_kind(), TokenKind::Kw(k) if k == kw) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn match_kw_opt(&mut self) -> Option<Kw> {
        if let TokenKind::Kw(kw) = self.peek_kind() {
            let k = kw;
            self.advance();
            Some(k)
        } else {
            None
        }
    }

    fn match_token(&mut self, kind: TokenKind) -> bool {
        if self.check(kind.clone()) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Path separator `::` (two `Colon` tokens), for `use rust::serde_json { … }`.
    fn match_colon_colon(&mut self) -> bool {
        if self.check(TokenKind::Colon) && self.peek_kind_at(1) == TokenKind::Colon {
            self.advance();
            self.advance();
            true
        } else {
            false
        }
    }

    fn check(&self, kind: TokenKind) -> bool {
        self.peek_kind() == kind
    }

    fn peek_kind(&self) -> TokenKind {
        self.tokens
            .get(self.pos)
            .map(|t| t.kind.clone())
            .unwrap_or(TokenKind::Eof)
    }

    fn peek_kind_at(&self, offset: usize) -> TokenKind {
        self.tokens
            .get(self.pos + offset)
            .map(|t| t.kind.clone())
            .unwrap_or(TokenKind::Eof)
    }

    fn advance(&mut self) -> Token {
        let t = self.tokens[self.pos].clone();
        if !matches!(t.kind, TokenKind::Eof) {
            self.pos += 1;
        }
        t
    }

    fn current_start(&self) -> u32 {
        self.tokens.get(self.pos).map(|t| t.start).unwrap_or(0)
    }

    fn previous_start(&self) -> u32 {
        if self.pos == 0 {
            0
        } else {
            self.tokens[self.pos - 1].start
        }
    }

    fn previous_end(&self) -> u32 {
        if self.pos == 0 {
            0
        } else {
            self.tokens[self.pos - 1].end
        }
    }

    fn previous_span(&self) -> Span {
        Span::new(self.previous_start(), self.previous_end())
    }

    /// Point-free section (#89): `.name` → `|_sec| _sec.name`;
    /// `.magnitude()` / `.scale(2.0)` → `|_sec| _sec.magnitude()` / `|_sec| _sec.scale(2.0)`.
    /// Extra method args are baked into the section; they are not extra lambda params.
    fn parse_point_free_section(&mut self, start: u32) -> Result<Expr, ParseError> {
        self.advance();
        let field = self.expect_ident()?;
        let recv = Ident::new("_sec", Span::new(start, field.span.end));
        let field_expr = Expr {
            kind: ExprKind::Field {
                base: Box::new(Expr {
                    kind: ExprKind::Ident(recv.clone()),
                    span: recv.span,
                }),
                field,
            },
            span: Span::new(start, self.previous_end()),
        };
        let body = if self.check(TokenKind::LParen) {
            self.advance();
            let args = self.parse_args()?;
            self.expect(TokenKind::RParen)?;
            Expr {
                kind: ExprKind::Call {
                    func: Box::new(field_expr),
                    args,
                },
                span: Span::new(start, self.previous_end()),
            }
        } else {
            field_expr
        };
        let span = Span::new(start, self.previous_end());
        Ok(Expr {
            kind: ExprKind::Lambda {
                params: vec![Param {
                    lifetime: None,
                    ownership: None,
                    name: recv,
                    ty: None,
                    span,
                }],
                body: Box::new(body),
            },
            span,
        })
    }

    fn unexpected(&self, expected: &'static str, found: TokenKind) -> ParseError {
        ParseError::Unexpected {
            expected,
            found,
            pos: self.current_start(),
            help: None,
        }
    }
}

fn kw_bytes(_kw: Kw) -> &'static [u8] {
    b"keyword"
}

trait ItemSpan {
    fn span(&self) -> Span;
}

impl ItemSpan for Item {
    fn span(&self) -> Span {
        match self {
            Item::Function(f) => f.span,
            Item::TypeDef(t) => t.span,
            Item::TraitDef(t) => t.span,
            Item::ShapeDef(s) => s.span,
            Item::Impl(i) => i.span,
            Item::Use(u) => u.span,
            Item::Const(c) => c.span,
            Item::Extern(e) => e.span,
            Item::Test(t) => t.span,
            Item::TestCompileFail(t) => t.span,
        }
    }
}
