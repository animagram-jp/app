# animagram-jp/app

A general-purpose boilerplate for GUI applications that edit and read structured data.
The application owns a single event loop and is designed to run on multiple host environments, such as WebAssembly in a browser.

構造を持つデータを編集・閲覧する、一般的な用途を想定したアプリケーションシステムのボイラープレート。
アプリ内にイベントループを持ち、複数のホスト環境(ブラウザのWebAssemblyなど)で動作するよう設計されている。

## Version

| Version | Status | Date | Description |
|-|-|-|-|
| 0.0.7   | current   | 2026-10-05 | For UI Test. |
| 0.1.0   | scheduled | 2026-11-31 | 1st release |

This project adheres to [Semantic Versioning](https://semver.org/).

---

## Apps

Applications built on this boilerplate. They share the same core.

このボイラープレート上に構築されたアプリケーション。同じコアを共有する。

| App | Description | 説明 |
|-|-|-|
| [calendar](#calendar) | Web calendar with reservation management. | 予約管理を含むWebカレンダー。 |
| [coc7th](#coc7th) | Call of Cthulhu 7th edition playing tool. | 新クトゥルフ神話TRPGをオンラインで遊ぶためのツール。 |

### Calendar

A web calendar with day, 3-day, week, and month views. Supports recurring events, reservation management, and JSON import and export.

日・3日・週・月の表示を持つWebカレンダー。繰り返し予定、予約管理、JSONのインポート・エクスポートに対応する。

### CoC7th

An online tool dedicated to Call of Cthulhu TRPG 7th Edition. Supports character sheet editing, dice rolling during sessions, and post-session log management and tallying.

新クトゥルフ神話TRPGに特化したオンラインツール。キャラクターシートの編集、セッション時のダイスロール、セッション後のログ管理と集計が可能。

---

## License

SPDX-License-Identifier: Apache-2.0
Copyright (c) 2026 Andyou <andyou@animagram.jp>

## Notice

"Call of Cthulhu" is a trademark of Chaosium Inc.
This project is an independent, unofficial work and is not affiliated with, endorsed by, or sponsored by Chaosium Inc.
『クトゥルフ神話TRPG』は Chaosium Inc. の著作物です。
『新クトゥルフ神話TRPG』は、株式会社アークライトによる翻案のもと株式会社KADOKAWAが発行しています。
本ソフトウェアの機能は同作のプレイを支援する非公式のオンラインツールであり、上記各社及びChaosium Inc.による公認・提携・後援を受けたものではありません。
