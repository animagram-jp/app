# Calendar

過去と未来、そして連続の3つの概念が定義する「時(time)」は、人間にとって、物事の固有と同一の源だ。
しかしながら、有限に表現する際には、必ず原点と最小単位からなる離散座標を必要とする。
現代社会においてそれは汎用的に秒(second)であり、用途ごとに十進小数から選ぶことで、座標間の互換性を保っている。
カレンダーアプリケーションは、更にその上で、年・月・週・曜日・日・時・分・秒・からなる暦・時法

## Temporal(時系概念)

- time line: 未来と過去の二項対立に依拠した正負方向を持つ連続した直線形状の概念。
- time length: time lineの正負方向に従って量を表す概念。
- time calendar: 原点epochと、lengthの最小単位secondによって定義された時系の1次元座標。
- time epoch: calendarの原点(datum.originに該当する)。
- time stamp: 時刻。暦に従って有限量の値をとる。
- time unit: year, month, day, hour, minutes, second: 階層関係付き離散単位、second以外への変換時の修正値(関数)を示すtimezoneとその標準値utcからなる。さらにこの上に、youbi (day of the week)や Nth week in the month の座標が構築されている。

- `time::stamp: [u8; 8]`: 4桁のyear以下、0.01 secondまでを保持したデータレイアウト。year, month, dayの値が0..0である時、値はtimestampのパターンを意味する。hour以下、最小単位から連続して0..0の値を持つ時、それはNone(ワイルドカード)として解釈できる。

## Specification

- 暦は人間のための系であるから、その生理特性に合わせ、Length::Minutes(5)を最小幅とする。

- 携帯サイズでもノート・据え置きサイズでも共通して、5行x7列の35日分(月次)をメインUIにできるよう設計する。
    - 5行に固定する代わりに、上下スクロールで1週ずつ表示範囲を移動する。
- オフラインでも同期以外の動作を提供し、ネットワーク機能は端末間同期・ユーザー間共有だけを行う。
- リリースノート, ログイン, メイン, モーダルを用意する。
- 入力データ(entry)には所有者ユーザーを定義し、所有者は更新しない。
    - ユーザー間共有機能における認可の判断材料は常に入力データの外に置く。
- iCalデータ(RFC5545)のインポート/エクスポート機能を持つ。

### Grid feature

- いわゆるreservationやappointmentと言われるグリッド管理機能は、calendarに最小単位を置いて作るtimeslotや、youbiをその他resoruce(locationやassignee)と共に管理するもので、より上位層として時系データ管理とは分離される。
- 暦は人間のための系であるから、timeslotもその生理特性に対して妥当な粒度を持つべきである。従って、15分間を最小粒度とする。

---

## Memorandum

メモアプリ。共同編集モデル(WhiteBoard)と通常のMemorandumモデルは、切り替え可能な排反モデルとする。
組版システムやtextarea、inline editorスタイリングなどが関係。

---

## Concept

- user: クライアントを通じてログインし得る主体。
    - preference: userに紐づく設定値
- event: 現実で起きた出来事。システム内では扱わない
- entry: システムがUI表示時に扱う、UIと1対1に対応したランタイムデータ
- resource: 電子計算機がentryを内部で解釈処理するためのストアデータ群
    - schedule: 複数のentryを作成するためのリソース
    - record: 1つのentryを作成するためのリソース。scheduleとの抵触時は、patchとして機能する
- state: プロセスがpattern群から特定のentryを構成するためのコンテクストデータ

- sort.youbi: u3 // 001 Monday 111 Sunday // start of week
- format.timestamp: u3 // 001 YYYY-M-D h:m 010 YYYY年M月D日 h時m分
- html.scope.main: u3 // year, month, week, 3 days, day
- drawer: u2 // 00 hidden 01 right 10 left
- modal: u2 // 00 hidden 01 show
- focused: u32 // html elements id
- calendar | selected: u22 // <- entry (2^22 > 24*(60/15)*365*80)

- RRULEについて: u64: timestampは、各上位要素を0..0とすることで、範囲絞り込み可能になる。これを利用し、time_range: [start: Timestamp, end: Timestamp]の形のまま繰り返し情報を保持可能ではないか。

```yaml
# Value 0...0 means null in each field.
# 1 byte = 1 octet = 8 bits.

handler:
    now:
    display:
        locale:
        color-scheme: [u8;1]
        grid:
            timeunit: # 10 minutesを最小とする離散値
            axis:
                # map()にて、rectgridのx軸と1対1変換関数を要する
                sort:
            time_range:
    filestores: Vec<FileStore>
    entry: DataStruct
    
    
    sync:
        status: # synced(client only, server only, both all)
        user:

entry:
    owner: 
    input:
        start: Timestamp
        end: Timestamp
        title: utf-8(~200bytes)
        text: utf-8(~8000bytes)
        url: # todo
        location: #todo
        tag: [u8;5]
        todo: u8(none, undone, done)
        layer: u4(none, 1~15)
    alerm:
        baseline:
        offset:
            sign:
            time_volume:

tag:
    owner: 
    type: # color, title, free
    parent_tag:
    value: [u8] # ~100bytes
    color: [u8; 3]
```

## 機能

- header: 前後範囲送り, preference選択
- modal fieldset: entry編集時に使うfieldと同時に、設定編集に使うfieldを追加。
    - email/password: hidden→表示切り替えに変更
    - preference: locale, color(theme), youbi, datetime, main_scope
    - tag一覧 × 最大512件: name/type/parent/color/削除ボタン、追加ボタン

settings field仕様:
- locale: select — 0001 EN / 0010 JA
- color(theme): select — 001 light_01 / 101 dark_01
- youbi: select — 001 月〜111 日
- datetime: select — 001 YYYY-M-D h:m / 010 YYYY年M月D日 h時m分
- main_scope: radio — 001 year / 010 month / 011 day / 100 hour
- tag.name: input[text] — 32byte pool参照
- tag.type: radio — 001 color / 010 title / 011 free
- tag.parent: select — tag index（0=root）
- tag.color: input[color] — RGB 24bit

```html
<header>
    <button type="button">キャンセル</button>
    <button type="submit">保存</button></header>

    <label hidden>
        <span>メール</span>
        <input type="email" name="" autocomplete="email"></label>
    <label hidden>
        <span>パスワード</span>
        <input type="password" name="" autocomplete="current-password"></label>
    <label>
        <span>タイトル</span>
        <input type="text" name=""></label>
    <label>
        <span>開始</span>
        <input type="date" name="">
        <input type="time" name=""></label>
    <label>
        <span>終了</span>
        <input type="date" name="">
        <input type="time" name=""></label>
    <label>
        <span>カラータグ</span>
        <input type="text" name=""></label>
    <label>
        <span>タイトルタグ</span>
        <input type="text" name=""></label>
    <label>
        <span>フリータグ</span>
        <input type="text" name=""></label>
    <label>
        <span>Todo</span>
        <input type="radio" name="" value="01">
        <input type="radio" name="" value="10"></label>
    <button type="button">テキスト+</button>
    <button type="button">URL+</button>
    <button type="button">場所+</button>
    <label hidden>
        <span>テキスト</span>
        <textarea name="" rows="3"></textarea></label>
    <label hidden>
        <span>URL</span>
        <input type="url" name=""></label>
    <label hidden>
        <span>場所</span>
        <input type="text" name="" autocomplete="off"></label>
    <button type="button">繰り返し+</button>
    <fieldset hidden>
        <label>
            <span>間隔</span>
            <input type="number" name="" value="1" min="1" max="9">
            <select name="">
                <option value="001">月</option>
                <option value="010">週</option>
                <option value="011">日</option>
                <option value="100">時間</option></select></label>
        <label>
            <span>曜日指定</span>
            <select multiple name="">
                <option value="001">月</option>
                <option value="010">火</option>
                <option value="011">水</option>
                <option value="100">木</option>
                <option value="101">金</option>
                <option value="110">土</option>
                <option value="111">日</option></select></label>
        <label>
            <span>有効期限</span>
            <input type="date" name=""></label></fieldset>
```

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
