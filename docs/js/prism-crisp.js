/* Crisp language for Prism — surface closest to Rust keywords + Elixir-ish defs,
   with Crisp-specific :=, ++, -- / {- -} comments, ambient !, test_compile_fail. */
(function (Prism) {
  Prism.languages.crisp = {
    comment: [
      {
        pattern: /\{-[\s\S]*?-\}/,
        greedy: true,
      },
      {
        pattern: /(^|[^:])--.*/,
        lookbehind: true,
        greedy: true,
      },
    ],
    string: {
      pattern: /"(?:\\.|[^\\"])*"/,
      greedy: true,
      inside: {
        interpolation: {
          pattern: /\{[^{}\n]+\}/,
          inside: {
            punctuation: /^\{|\}$/,
            'interpolation-content': {
              pattern: /[\s\S]+/,
              inside: null, // filled below
            },
          },
        },
        escape: /\\./,
      },
    },
    keyword:
      /\b(?:async|await|break|catch|continue|do|else|extern|for|if|impl|in|loop|match|mod|mut|own|panic|pub|rc|arc|ref|return|shape|shared|spawn|test_compile_fail|test|then|throw|trait|type|unsafe|use|while|with|as)\b/,
    boolean: /\b(?:true|false|none|some)\b/,
    builtin:
      /\b(?:int|uint|float|bool|char|str|vec|map|set|Self|self|super|crate)\b/,
    number: /\b\d[\d_]*(?:\.\d[\d_]*)?(?:[eE][+-]?\d+)?\b/,
    function: /\b[a-z_][\w']*(?=\s*\()/,
    operator:
      /:=|\+\+|>>>|\|\|\||=>|->|\.\.=|\.\.|\*\*|<<|>>|<=|>=|==|!=|&&|\|\||[+\-*/%=<>!|&^~?:]/,
    punctuation: /[{}[\];(),.]/,
    "class-name": /\b[A-Z][\w']*\b/,
  };

  // Allow nested Crisp highlighting inside string interpolations.
  Prism.languages.crisp.string.inside.interpolation.inside[
    'interpolation-content'
  ].inside = Prism.languages.crisp;
})(Prism);
