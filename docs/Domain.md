// This file includes untranslated text (ja).

# ドメイン

システムは、人間とコンピューターという二つの与条件を持つ。もう1つの与条件として、システムが委託を受ける、人間の特定の働きの集合を、ドメインと名付ける。ドメインに従ってコンピューターがデータを認識し、処理する仕組みを、「データモデルスキーマ定義」によって可能にすることを考える。

## データモデルとは

- システムは人間の概念に沿った機能を要求するが、コンピューターが実現できるのは、データの入力・処理・参照・保存と言った、データに対する操作である。そこで、ドメインを構成する概念のうち、時間空間的に連続性のある概念群をモデルと呼び、時間空間的に連続しない概念を、モデルに対する操作として表現する。モデル同士は、定義・非定義に基づく多対一の関係を結ぶ(A subject is defined by a set of predicates.)。設計原則として、非定義者の根に立つ単一モデルから、定義者方向へ逆転を廃した木構造として成立させる。これには、ドメインを構成する概念の構造的理解を必要とする。

- コンピューターのデータ保持の都合から、単一の根を非定義者とするモデル群をドメイン内に複数設計する必要がある。これら、複数のモデル、モデルグループの関係は、処理手続きとして表現する。この実行体定義も、ドメインの構成物である。

- スクリプト上での対応は、以下の通り。
    - ドメイン: モジュール(名前空間)
    - モデルグループ: モジュール
    - モデル群: 関数定義が所属するオブジェクト
    - ハンドラー: データストラクト(インスタンスidと、インスタンス内部でのidを指定してデータを編集できるオブジェクト)とモデル関数を使って、イベントに対応した編集処理を定義されたオブジェクト

```rust

pub enum Lang {En(En), Ja};
pub enum En {Us, Uk};

// "domain"という例示用ドメイン
mod domain {
    // "model"という例示用モデルグループ
    mod model {
        // "model"グループ単一の被定義根モデル
        pub enum Model {
            // "model"モデルの定義者である"predicate"例示用モデル
            Predicate,
            // ユーザーカスタムの実データ格納用id帯域
            Custom,
        }
        impl Model {

            pub fn ids(&self) -> &[u32] { // 子に一意な実データ格納用idを配分
                match Self {
                    Self::Predicate => &[4],
                    Self::Custom => &[5..100],
                }
            }

            pub fn display(&self, lang: Lang) -> & 'static str {
                match (self, lang) {
                    (Self::Predicate, _) => "predicate",
                    (Self::Custom, _)   =>  "custom", // 使うのはデバッグの時だけなので、消した方が良いかも
                }
            }
        }

        // 定義者構成の無い終端モデルの例
        pub struct Predicate;

        impl Predicate {
            // データストラクトに格納されたidによるバイト列集合を取得しモデルに要求される全情報の原型として最大公約数となる関数。ここでは単一の&strとする。
            pub fn read(data_struct: &DataStruct) -> &str {
                data_struct.get(Model::Predicate::ids().0).as_str
            }

            pub fn write(data_struct: 'a &mut DataStruct, value: &str) -> 'a &mut DataStruct {
                _result = data_struct.set(Model::Predicate::ids().0, value);
                data_struct
            }

            // ～の像を得る
            // Modelが他のModelからの要求を主体的に契約するための、externalの最大公約数の解釈
            pub fn project() -> String {

            }

            // ～を導出する
            // Modelが他のModelのproject()を要求する時、それをderive()の実行に集約する
            pub fn derive(instance: 'a &mut Datastruct) -> 'a &mut Datastruct {
                
            }
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

        fn map() {

        }
    }
}
```
