# Calendar

過去と未来、そして連続の3つの概念が定義する「時(time)」は、人間にとって、物事の固有と同一の源だ。
しかしながら、有限に表現する際には、必ず原点と最小単位からなる離散座標を必要とする。
TAIに代表されるデジタル時刻システムにおいて、最小単位は秒(second)であり、用途ごとに十進小数から選ぶことで、座標間の互換性を保っている。
カレンダーアプリケーションでは、更にその上に、年・月・週・日・時・分・秒・タイムゾーンからなる暦・時法を設ける。これにより、月・週・日(・曜日)・時・分・秒は、人間に理解しやすい有界離散な値域の座標(D次元グリッド)を構築する。

## 時系の概念

- time line: 未来と過去の二項対立に依拠した正負方向を持つ連続した直線形状の概念。
- time length: time lineの正負方向に従って量を表す概念。
- time system: 原点と、最小量単位secondによって定義された時系の1次元座標。
- time calendar: 暦・時法。time systemには無い変数として、タイムゾーンを要求する。
- time stamp: 時刻。暦・時法に従って有限量の値をとる。
- time unit: year, month, day, hour, minutes, second: 階層関係付き離散単位、second以外への変換時の修正値(関数)を示すtimezoneとその標準値utcからなる。さらにこの上に、youbi (day of the week)や Nth week in the month の座標が構築されている。

- `time::stamp: [u8; 8]`: 4桁のyear以下、0.01 secondまでを保持したデータレイアウト。year, month, dayの値が0..0である時、値はtimestampのパターンを意味する。hour以下、最小単位から連続して0..0の値を持つ時、それはNone(ワイルドカード)として解釈できる。

## その他の概念

- event: 現実で起きた出来事。入力されるentryとは区別される。
- entry: システムがUI表示時に扱う、UIと1対1に対応したランタイムデータ
- resource: 電子計算機がentryを内部で解釈処理するためのストアデータ群
    - recurrence: 複数のentryを作成するためのリソース
    - record: 1つのentryを作成するためのリソース。recurrenceとの抵触時は、patchとして機能する

## Specification

- 人間の生理特性に合わせ、時間量の入力は、5分の倍数とする。

- 携帯サイズでもノート・据え置きサイズでも共通して、5行x7列の35日分(月次)をメインUIにできるよう設計する。
    - 5行に固定する代わりに、上下スクロールで1週ずつ表示範囲を移動する。
- オフラインでも同期以外の動作を提供し、ネットワーク機能は端末間同期・ユーザー間共有だけを行う。
- 入力データ(entry)には所有者ユーザーを定義し、所有者は更新しない。
    - ユーザー間共有機能における認可の判断材料は常に入力データの外に置く。
- iCalデータ(RFC5545)のインポート/エクスポート機能を持つ。

### Grid feature

- いわゆるreservationやappointmentと言われるグリッド管理機能は、calendarに最小単位を置いて作るtimeslotや、youbiをその他resource(locationやassignee)と共に管理するもので、より上位層として時系データ管理とは分離される。
- 暦は人間のための系であるから、timeslotもその生理特性に対して妥当な粒度を持つべきである。従って、15分間を最小粒度とする。

---

```yaml
handler:
    color-scheme:
    language:
    locale:
    now:
    view:
        scope:
        axis: # rectgridのUnit座標との互換関数axis<D: u8>.from()/to()を要する
            sort:
    filestores: Vec<FileStore>
    entry: DataStruct
    
    sync:
        user:
        status: # synced(client only, server only, both all)

entry: # section / fieldset
    owner: 
    start: time::stamp
    end: 
    title: utf-8(~200bytes)
    text: utf-8(~8000bytes)
    url:
    location:
    tag: [u8;5]
    todo: none, undone, done
    layer: none, 1~15
    alarm:
        baseline:
        offset:
            sign:
            time_volume:
            
tag:
    owner: 
    type: # color, title, free
    parent_tag: # 0=root
    value: [u8] # ~100bytes
    color: [u8; 3]
```

---

## Memorandum

メモアプリ。共同編集モデル(WhiteBoard)と通常のMemorandumモデルは、切り替え可能な排反モデルとする。
組版システムやtextarea、inline editorスタイリングなどが関係。

- コマンド群(e.g., フォント変更)を非表示にできる (思考と1つの動作に集中できる)
- タイトルにカーソルの初期位置が当たらず、デフォルトで自動埋めも起こらない
- ポモドーロタイマーを設定できる。キリの良い時間(e.g., 1分)ごとに表示される。
- 削除するハードルを上げるための工夫がある。履歴保持など。
- 覗き込まれた時やスクリーンシェアをしやすいよう、一覧や検索結果に出さないプライベート設定ができる。
- 公開設定のトグルがある。
- 横にpx単位でメモを設置できる。

- [sngazm: sate](https://github.com/sngazm/sate)
- [橋本麦: 構造化しないUI](https://baku89.com/ja/%E6%A7%8B%E9%80%A0%E5%8C%96%E3%81%97%E3%81%AA%E3%81%84UI)

---

# RFC 5545

## VCALENDAR

BEGIN:VCALENDAR
END:VCALENDAR
VERSION:2.0
PRODID:animagram.jp

BEGIN:VEVENT,VTODO,VJOURNAL,VFREEBUSY,VTIMEZONE,VALARM
END:VEVENT,VTODO,VJOURNAL,VFREEBUSY,VTIMEZONE,VALARM

CALSCALE:GREGORIAN
METHOD:REQUEST/PUBLISH/REPLY

## ISO 8601 Basic Format

YYYYMMDD"T"HHMMSS"Z" 　　　　　　　　// utc
YYYYMMDD"T"HHMMSS    　　　　　　　　// local
TZID=Asia/Tokyo:YYYYMMDD"T"HHMMSS // TZID local

## VEVENT

DTSTAMP:UTC      // インポートとエクスポート時のみ値を保持
UID:             // random 128 bitsをasciiに変換出力(uuid)
DTSTART:         // イベントの開始時刻

DTEND:           // イベントの終了時刻
DURATION:

CLASS:PUBLIC/PRIVATE/CONFIDENTIAL
CREATED:
DESCRIPTION:
GEO:
LAST-MOD:
LOCATION:
ORGANIZER:
PRIORITY:0-9
SEQ:
STATUS:
SUMMARY:
TRANSP:OPAQUE/TRANSPARENT
URL:
RECURID:

RRULE:
  FREQ:SECONDLY/MINUTELY/HOURLY/DAILY/WEEKLY/MONTHLY/YEARLY
  INTERVAL:1~
  COUNT:1~
  UNTIL: 終了日時
  BYDAY:MO,TU,WE,TH,FR,SA,SU
  BYMONTHDAY:月内の日
  BYMONTH:月
  WKST:週の開始曜日

ATTACH:
ATTENDEE:
CATEGORIES:
COMMENT:
CONTACT:
EXDATE:
RSTATUS:
RELATED:
RESOURCES:
RDATE:
X-PROP:
IANA-PROP:

### VTODO

DTSTAMP:
UID:

DTSTART:
DUE:
COMPLETED:完了日時
PERCENT-COMPLETE:進捗率（0-100）
STATUS:NEEDS-ACTION/COMPLETED/IN-PROCESS/CANCELLED
SUMMARY:
DESCRIPTION:
PRIORITY:0-9

## VJOURNAL

DTSTAMP:
UID:

DTSTART:
SUMMARY:
DESCRIPTION:

## VFREEBUSY

DTSTAMP:
UID:

DTSTART:
DTEND:
FREEBUSY:
ORGANIZER:
ATTENDEE:

## VTIMEZONE

TZID:
STANDARD:
DAYLIGHT:

## VALARM

ACTION:AUDIO/DISPLAY/EMAIL
TRIGGER:
DURATION:
REPEAT:
ATTACH;

## property parameter

- `TZID`: タイムゾーン
- `VALUE`: 値の型（DATE, DATE-TIME等）
- `LANGUAGE`: 言語
- `CN`: Common Name（表示名）
- `ROLE`: 役割（CHAIR, REQ-PARTICIPANT, OPT-PARTICIPANT）
- `PARTSTAT`: 参加ステータス（NEEDS-ACTION, ACCEPTED, DECLINED, TENTATIVE）
- `RSVP`: 返信要求（TRUE/FALSE）

## escape & encoding

- 行の折り返し: 75オクテット超過時、次行を空白で開始
- 特殊文字エスケープ:
  - `,` → `\,`
  - `;` → `\;`
  - `\` → `\\`
  - 改行 → `\n`

```
DESCRIPTION:これは長い説明文です。非常に長いテキストは75文字で
 折り返されます。セミコロン\;やカンマ\,はエスケープします。
```

## reference

- RFC 5545: https://tools.ietf.org/html/rfc5545
- iCalendar.org: https://icalendar.org/
