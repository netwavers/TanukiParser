# TanukiParser CLI 取扱説明書 (CLI User Manual)

`TanukiParser` は、EBNF文法ファイルから各言語用のパーサーソースコードを生成するコマンドラインツールです。

## 🐾 基本コマンド

```powershell
.\TanukiParser.exe [オプション]
```

※開発環境では `cargo run -- [オプション]` で実行可能です。

---

## 🐾 コマンドライン引数 (Options)

| 短縮 | 完全名 | 引数 | 説明 |
| :--- | :--- | :--- | :--- |
| `-i` | `--input` | `<path>` | **必須**。入力となる `.ebnf` 文法ファイルのパス。 |
| `-o` | `--output` | `<path>` | **必須**。生成されたコードを保存するファイル名。 |
| `-t` | `--target` | `<lang>` | ターゲット言語 (`python`, `rust`, `csharp`)。デフォルトは `csharp`。 |
| `-h` | `--help` | - | ヘルプメッセージを表示します。 |
| `-V` | `--version` | - | バージョン情報を表示します。 |

---

## 🐾 使用例 (Examples)

### 1. Python用パーサーの生成
最も一般的な使用例です。
```powershell
.\TanukiParser.exe -i grammar.ebnf -o parser.py --target python
```

### 2. Rust用パーサーの生成
Rustプロジェクトの `src` ディレクトリ等に出力します。
```powershell
.\TanukiParser.exe -i grammar.ebnf -o src/parser.rs --target rust
```

### 3. C#用パーサーの生成
レガシーなOEBNF互換モードです。
```powershell
.\TanukiParser.exe -i grammar.ebnf -o Parser.cs --target csharp
```

---

## 🐾 エラーと診断 (Diagnostics)

TanukiParserは、解析中に文法のミスを見つけると詳細なレポートを出力します。

```text
Errors found during parsing:
  [12:24] Unexpected token: TAlternative
  [45:10] Rule 'undefined_rule' is referenced but not defined.
```

*   **[行:列]**: エラーが発生した正確な場所を示します。
*   **耐故障パース**: 参照エラー（未定義のルール呼び出し）があっても、パーサーのコード生成自体は継続されます。不整合な箇所はコード内で自動的にコメントアウトされ、まずは「ビルドが通る状態」のコードを出力します。

---

## 🐾 開発者へのアドバイス
文法を変更した後は、常にこのCLIを実行してパーサーを再生成してください。
バッチファイルやMakefileに組み込んでおくと、さらに便利に使えますわよ、ご主人様！🐾
