// fixed bits layout for pooling lists
// note:
// - Value 0...0 means null in each field.
// - // <- meanins idx

// state
// note:
// - ui, html: only for browser
//
// | category | field        | bit |
// |----------|--------------|-----|
// | system   | now          |  64 | // datetime // <- UT f64(js_sys::Date::now)
// |          | user         |  32 | // <- user
// | layout   | mode         |   2 | // future function
// |          | locale       |   4 |
// |          | color        |   3 | // 001 light_01 101 dark_01
// | sort     | youbi        |   3 | // 001 Monday 111 Sunday // start of week
// | format   | datetime     |   3 | // 001 YYYY-M-D h:m 010 YYYY年M月D日 h時m分
// | html     | main_scope   |   3 | // 001 year 010 month 011 day 100 hour
// |          | drawer       |   2 | // 00 hidden 01 right 10 left
// |          | modal        |   2 | // 00 hidden 01 show
// |          | focused      |  32 | // html elements id
// | calendar | selected     |  22 | // <- entry (2^22 > 24*(60/15)*365*80)

// user
//
// | category   | field         | bit  |
// |------------|---------------|------|
// | attribute  | uuid          |  128 |
// |            | email         | 2032 |
// |            | password_hash |   32 | // <- password_hash
// | preference | mode          |    1 |
// |            | locale        |    4 |
// |            | color         |    3 |
// |            | youbi         |    3 | 
// |            | datetime      |    3 |
// |            | main_scope    |    3 |

// pooling lists
//
// server-only
//
// email (variable bits)    idx < 2^32 // max 2032(8 x 254) bits
// password_hash (512 bits) idx < 2^32
//
// common
//
// 

// entry
//
// | category | field        | bit |
// |----------|--------------|-----|
// | system   | resource     |  44 | // record index | schedule index
// |          | owner        |  32 |
// |          | sync_status  |   2 | // 01 browser only 10 server only 11 synced
// | input    | start        |  64 | // datetime
// |          | end          |  64 | // datetime
// |          | title        |  22 | // title index 1600 (32*50)
// |          | text         |  22 | // text index 64000 (32*2000)
// |          | url          |  46 | // url (scheme+host_idx+path_offset)
// |          | location     |  32 | // location pool file byte offset
// |          | tag          |  45 | // tag index(9bit) x 5
// |          | todo         |   2 | // 00 null 01 undone 10 done
// |          | hierarchy    |   4 | // 0001 1 1010 10 (1011 11 1111 15)
// | repeat   | frequency    |   3 | // 001 monthly 010 weekly 011 daily 100 hourly
// |          | interval     |   5 |
// |          | youbi        |  21 | // 3*7
// |          | until        |  64 | // datetime
// | alarm    | triggers     |  32 | // alarm trigger index x 4

// tag (80 bits)
//
// note:
// - entry は (owner_id, tag_index) ペアで参照
// - parent = 0 はルート（親なし）
// - type: color-tag はカレンダー全体の色分け、title-tag はタイトル埋め込み、free はその他
// - type/hierarchy は設定画面から変更可
// - name は固定長32byte pool（UTF-8、null padding）、index=9bit
// - 1ユーザーあたり最大512個（9bit）
//
// | category | field   | bit |
// |----------|---------|-----|
// |          | owner   |  32 | // user index
// |          | type    |   3 | // 001 color  010 title  011 free
// |          | name    |   9 | // fixed pool index (32byte/entry)
// |          | parent  |   9 | // tag index (0 = root)
// |          | color   |  24 | // RGB
// |          | padding |   3 |

// alarm trigger
//
// | category | field    | bit |
// |----------|----------|-----|
// |          | baseline |   2 | // 01 start 10 end
// |          | sign     |   2 | // 01 increment 10 decrement
// | offset   | day      |   5 |
// | offset   | hour     |   5 |
// | offset   | minute   |   6 |

// url (46bit)
//
// | category | field        | bit |
// |----------|--------------|-----|
// |          | scheme       |   2 | // 01 http 10 https
// |          | host_idx     |  12 | // fixed pool index
// |          | path_offset  |  32 | // variable pool byte offset

// record
//
// | category | field        | bit |
// |----------|--------------|-----|
// | system   | uuid         | 128 |
// |          | owner        |  32 |
// |          | sync_status  |   2 |
// | input    | start        |  64 |
// |          | end          |  64 |
// |          | title        |  22 |
// |          | text         |  22 |
// |          | url          |  46 |
// |          | location     |  32 | // location pool file byte offset
// |          | tag          |  45 | // tag index(9bit) x 5
// |          | todo         |   2 |
// |          | hierarchy    |   4 |
// | alarm    | triggers     |  32 |

// schedule
//
// | category | field        | bit |
// |----------|--------------|-----|
// | system   | owner        |  32 |
// |          | sync_status  |   2 |
// | input    | start        |  64 |
// |          | end          |  64 |
// |          | title        |  22 |
// |          | text         |  22 |
// |          | url          |  46 |
// |          | location     |  32 | // location pool file byte offset
// |          | tag          |  45 | // tag index(9bit) x 5
// |          | todo         |   2 |
// |          | hierarchy    |   4 |
// | repeat   | frequency    |   3 |
// |          | interval     |   5 |
// |          | youbi        |  21 |
// |          | until        |  64 |
// | alarm    | triggers     |  32 |

// --- url ---
pub const OFFSET_URL_SCHEME:      u64 = 44;
pub const OFFSET_URL_HOST_IDX:    u64 = 32;
pub const OFFSET_URL_PATH_OFFSET: u64 =  0;

pub const MASK_URL_SCHEME:      u64 = 0x3;
pub const MASK_URL_HOST_IDX:    u64 = 0xFFF;
pub const MASK_URL_PATH_OFFSET: u64 = 0xFFFFFFFF;

pub const URL_SCHEME_HTTP:  u64 = 0b01;
pub const URL_SCHEME_HTTPS: u64 = 0b10;

// --- locale ---
pub const LOCALE_NULL: u64 = 0b0000;
pub const LOCALE_EN:   u64 = 0b0001;
pub const LOCALE_JA:   u64 = 0b0010;

// --- todo ---
pub const TODO_NULL:   u64 = 0b00;
pub const TODO_UNDONE: u64 = 0b01;
pub const TODO_DONE:   u64 = 0b10;

// --- sync_status ---
pub const SYNC_BROWSER_ONLY: u64 = 0b01;
pub const SYNC_SERVER_ONLY:  u64 = 0b10;
pub const SYNC_SYNCED:       u64 = 0b11;