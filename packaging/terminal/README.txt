Overseer terminal — standalone AI assistant

macOS/Linux: sh ./install-terminal.sh
Windows x64: run .\install-terminal.ps1 in PowerShell.
Installers verify the extracted files and retain existing application data.
Default location: ~/.local/bin (Windows: %LOCALAPPDATA%\Overseer\bin).
Add this directory to your shell's PATH, then run: overseer-tui
You can also launch the executable directly from this folder.

Press F1 for help. Ctrl+O lists your installed Ollama models.
Use Settings for cloud providers and API keys. No model is downloaded for you.
Ollama/cloud accounts are separate prerequisites; models are not included.
For a remote daemon: overseer-tui --remote --url http://127.0.0.1:8080

Linux build targets Ubuntu 24.04 or compatible systems; audio requires libasound2.
Mac builds are currently unsigned/not notarized: this is a preview release.
Public launch remains subject to the launch-readiness review.
Uninstall: remove overseer-tui from the install directory. This keeps your data.
Source and issues: https://github.com/Revaycolizer/overseer
