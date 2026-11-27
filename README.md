[![CI](https://github.com/animagram-jp/app/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/animagram-jp/app/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/animagram-jp/app/graph/badge.svg)](https://codecov.io/gh/animagram-jp/app)
[![OpenSSF Scorecard](https://api.scorecard.dev/projects/github.com/animagram-jp/app/badge)](https://scorecard.dev/viewer/?uri=github.com/animagram-jp/app)
[![OpenSSF Best Practices](https://www.bestpractices.dev/projects/16653/badge)](https://www.bestpractices.dev/projects/16653)
[![OpenSSF Baseline](https://www.bestpractices.dev/projects/16653/baseline)](https://www.bestpractices.dev/projects/16653)

A general-purpose boilerplate for GUI applications that edit and read structured data.
The application owns a single event loop and is designed to run on multiple host environments, such as WebAssembly in a browser.

---

構造を持つデータを編集・閲覧する、一般的な用途を想定したアプリケーションシステムのボイラープレート。
アプリ内にイベントループを持ち、複数のホスト環境(ブラウザのWebAssemblyなど)で動作するよう設計されている。

# Requirements

人間に普遍的に必要とされるアプリケーションを、提供コストをユビキタスに成り得る閾値まで抑えたWebシステムアーキテクチャで実現する。普遍的機能とは、以下を指す:

1. データを編集し、保存・複数端末で同期する機能。データは、その最適な閲覧・編集UIを決定するスキーマに多対一に紐づく。人間及びシステムにとって、時系が原始のデータの識別手段である。既存のアプリで「カレンダー」「メモ」に対応する機能は、人間の意識に昇る時系であるかの違いと理解できる。
2. 任意のスキーマデータを編集する機能。
3. スキーマ自体を編集する機能。

開発は、カレンダーやCoCプレイツールなど、具体的なアプリケーションを普遍的なアーキテクチャで実装することで進行する。各アプリケーション固有の処理は、分離境界を明確にしてコンパイル・ビルド時に選択可能にする。各実装時は、普遍的な要件にエスカレーションする妥当性を毎回検討する。

# Apps

Applications built on this boilerplate. They share the same core.

このボイラープレート上に構築されたアプリケーション。同じコアを共有する。

| App | Description | 説明 |
|-|-|-|
| [calendar](#calendar) | Web calendar with reservation management. | 予約管理を含むWebカレンダー。 |
| [coc7th](#coc7th) | Call of Cthulhu 7th edition playing tool. | 新クトゥルフ神話TRPGをオンラインで遊ぶためのツール。 |

## Calendar

A web calendar with day, 3-day, week, and month views. Supports recurring events, reservation management, and JSON import and export.

日・3日・週・月の表示を持つWebカレンダー。繰り返し予定、予約管理、JSONのインポート・エクスポートに対応する。

## CoC7th

An online tool dedicated to Call of Cthulhu TRPG 7th Edition. Supports character sheet editing, dice rolling during sessions, and post-session log management and tallying.

新クトゥルフ神話TRPGに特化したオンラインツール。キャラクターシートの編集、セッション時のダイスロール、セッション後のログ管理と集計が可能。

# Notice

"Call of Cthulhu" is a trademark of Chaosium Inc.
This project is an independent, unofficial work and is not affiliated with, endorsed by, or sponsored by Chaosium Inc.
『クトゥルフ神話TRPG』は Chaosium Inc. の著作物です。
『新クトゥルフ神話TRPG』は、株式会社アークライトによる翻案のもと株式会社KADOKAWAが発行しています。
本ソフトウェアの機能は同作のプレイを支援する非公式のオンラインツールであり、上記各社及びChaosium Inc.による公認・提携・後援を受けたものではありません。
