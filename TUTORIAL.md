# TanukiParser チュートリアル (Tutorial)

このチュートリアルでは、Pythonを使用して「歌詞のタグ除去」を行うプログラムを例に、TanukiParserの使い方を学びます。

## 🐾 Step 1: 文法ファイルを準備する (`lyrics.ebnf`)

```ebnf
@namespace MyProject
@class LyricsParser
@decl list results

@token OPEN "["
@token CLOSE "]"
@token TEXT

@@
# タグ [ ... ] を読み飛ばし、それ以外のテキストを results に追加する
lyrics = ( tag | text_content )*

tag = OPEN ( TEXT | OPEN | CLOSE )* CLOSE
text_content = TEXT @{ self.results.append(self.token.content) @}
```

---

## 🐾 Step 2: パーサーコードを生成する

TanukiParserを使用して、Python用のコードを生成します。

```powershell
cargo run -- -i lyrics.ebnf -o lyrics_parser.py --target python
```

---

## 🐾 Step 3: プロジェクトに組み込む (Python)

生成されたパーサーを動かすには、簡単な `Lexer`（字句解析器）が必要です。

```python
# main.py
from lyrics_lexer import LyricsLexer  # 自分で実装するか、既存のものを流用
from lyrics_parser import LyricsParser

# 1. 歌詞データとレクサーを準備
text = "Hello [Intro] World"
lexer = LyricsLexer(text)

# 2. パーサーのインスタンスを作成
parser = LyricsParser(lexer)
parser.token = lexer.get_token() # 最初のトークンをセット

# 3. パース開始！
parser.parse()

# 4. 結果の確認
print("".join(parser.results)) # -> "Hello  World"
```

---

## 🐾 Step 4: さらに高度なこと

`@{ ... @}` 内では、ターゲット言語の全機能が使えます。
条件分岐を入れたり、外部ライブラリを呼び出したりして、高度なデータ抽出エンジンを作り上げましょう！

ご主人様、TanukiParserの力で開発をどんどん楽にしちゃいましょうね！🐾
