// This file includes untranslated text (ja).

# Domain

- updated at: 2026-06-29

　システムが認識するべき概念体系は、コンピューターと人間の2つどちらかに由来する。このうち人間に由来する概念は、実際のシステムを設計する上で、システムの先に在る普遍(例えば、言語)と、システムのライフタイム範疇で変質し得るもの(例えば、カルテ)に二分し、区別する必要がある。後者の概念体系群を、システムにおけるドメインと呼ぶこととする。

　オブジェクト: コンピューターに求める動作を表現するために、動作の目的語(関数の引数内)に集合名を与えたもの。この時、オブジェクトの名前空間に関数を定義するが、これは名前空間外のオブジェクトとの相関関係を実行するものであってはならない。オブジェクト間の関係は、処理体の実行順定義が最終的に表現する。
　ドメインを構成するオブジェクト群は、システムへの流入経路によって定義が与えられているもの(datum)と、それらの特定の集合が定義するもの(directory)に明確に分けられるが、この境界は、システムのライフタイム中、容易に変更され得るべきものなので、敢えてオブジェクトとして共通で扱う。

　従って、スクリプト上のドメインは、オブジェクト定義群だけでなく、その関係性を表現するためのオブジェクト外定義や、処理の手続き定義も必須の構成要素である。

　保存: 永続層を利用して情報を保持するには、id体系で操作可能なバイト列上に、1つのインスタンスとして重複の無いオブジェクトグループを対応させ、オブジェクトはそれぞれidの値と、読み書き関数を定義する必要がある。

　概念の構造体: 概念間に存在する関係は、被定義・定義に基づく一対多の関係である(A subject is defined by a set of predicates.)。設計原則として、非定義者の根に立つ単一モデルから、定義者方向へ逆転を廃した木構造として成立させる。これには、ドメイン内外への深い理解を要求する。

---

## Example

```rust
use app::{Lang, En};
use data_struct::DataStruct;

/// 被定義関係の終端オブジェクト
pub enum Subject {
    Predicate,
    Predicate,
    /// ユーザーカスタムの実データ格納用id帯域
    Custom,
}

impl Subject {
    /// id: 保存に必要なid: u32を効率的に重複なく定義側終端に配布するための関数
    ///
    /// base_id: predicate間の必要id数に差異が大きい場合に、offset単体で配布
    /// ids: 範囲列挙可能な場合に、スライスで配布
    /// id: 全predicateが1つまでしか必要としない場合に、定数で配布
    pub const fn base_id(&self) -> u32 {
        match Self {
            Self::Predicate => 4,
            Self::Custom    => 6,
        }
    }

    /// display: 定義者のデータに依存しない集合名は被定義者に所属する
    pub const fn display(&self, lang: Lang) -> &'static str {
        match (self, lang) {
            (Self::Predicate, Lang::En(_)) => "Predicate",
            (Self::Predicate, Lang::Ja)    => "定義者",
            (_, _) => "",
        }
    }

    /// list: オブジェクトの定義者の列挙も被定義者に所属する
    pub fn list(&self) -> &[Subject] {
        &[
            Self::Predicate,
        ]
    }
}

pub trait Predicate {
    fn ids(&self) -> &[u32];
    
}

/// 定義関係の終端オブジェクト
pub struct Predicate;

impl Predicate {
    const SUBJECT = Subject::Predicate;
    const BASE_ID = Subject::Predicate::base_id();

    pub fn ids() -> [u32; 3] {
        [
            BASE_ID,
            BASE_ID + 1,
            BASE_ID + 2, 
        ]
    }

    // let bytes <N: usize>: Result<[&[u8]; N], ListError> = (0..N)
    //     .map(|i| instance.get(i as u32))
    //     .collect::<Result<Vec<_>, _>>()
    //     .map(|v| v.try_into());
    // idからバイト列集合を取得しモデルに要求される全情報の原型として最大公約数となる関数。
    pub fn read(bytes: [&[u8]; 3]) -> (u8, u8, &str) {
        bytes
    }

    pub fn write(data_struct: 'a &mut DataStruct, value: (u8, u8, &str)) -> 'a &mut DataStruct {
        _result = data_struct.set(Self.BASE_ID, [value.0, value.1]);
        _result = data_struct.set(Self.BASE_ID + 1, value.2.as_bytes());
        data_struct
    }

    // ～の像を得る
    // 他のオブジェクトからの要求を主体的に契約するための、戻り値の最小公倍数
    pub fn project(p1: u8, p2: u8, p3: &str) -> (u8, u8, u8, String) {
        (
            p1,
            p2,
            p1 + p2,
            format!("{p3}"),
        )
    }

    // ～を導出する
    // オブジェクトが要求する、他のオブジェクトのprojectの実行を集約する
    pub fn derive(instance: 'a &mut Datastruct) -> 'a &mut Datastruct {

    }
}

pub struct Handler {
    instances: FileStore,
    data_struct: DataStruct,
}

impl Handler {

    pub async fn ready(store: FileStore, data_struct) -> Self {
        instances = store.open(),
        data_struct
    }

    pub fn close() -> Result {
        instances.close()
        ok()
    }

    pub fn process(event: Event, lang: Lang) -> Vec<Command> {
        match event {
            _ =>
        }
    }

    /// model::{Model, Predicate} <-> dom::Id
    fn to_dom() {

    }
    fn from_dom() {

    }
}
```
