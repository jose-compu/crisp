// Optional LanguageClient host for crisp-lsp (#56 / #57).
// Without `vscode-languageclient` installed, the extension still provides TextMate highlighting.

const vscode = require("vscode");

/**
 * @param {import('vscode').ExtensionContext} context
 */
async function activate(context) {
  const cfg = vscode.workspace.getConfiguration("crisp");
  if (!cfg.get("lsp.enabled", true)) {
    return;
  }
  let LanguageClient;
  let TransportKind;
  try {
    ({ LanguageClient, TransportKind } = require("vscode-languageclient/node"));
  } catch {
    console.log("crisp: vscode-languageclient not present — highlighting only");
    return;
  }
  const bin = cfg.get("lsp.path", "crisp-lsp");
  const serverOptions = {
    run: { command: bin, transport: TransportKind.stdio },
    debug: { command: bin, transport: TransportKind.stdio },
  };
  const clientOptions = {
    documentSelector: [{ scheme: "file", language: "crisp" }],
  };
  const client = new LanguageClient(
    "crisp",
    "Crisp Language Server",
    serverOptions,
    clientOptions
  );
  context.subscriptions.push(client);
  await client.start();
}

async function deactivate() {}

module.exports = { activate, deactivate };
