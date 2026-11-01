#[cfg(feature = "meet")]
pub mod meet;
#[cfg(feature = "sync")]
pub mod sync;

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub mod account;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub mod handler;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub mod host;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub mod http;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub mod session;

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
#[derive(Clone, Debug, PartialEq)]
pub struct Principal {
    pub user:   u32,
    pub device: u16,
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub enum Action {
    Read,
    Write,
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub enum Resource {
    Account(u32),
    Store { user: u32, name: alloc::string::String },
    Peer(u64),
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub fn authorize(principal: &Principal, action: Action, resource: &Resource) -> bool {
    let _ = (principal, action, resource);
    true
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub trait Open: crate::file_store::FileStore {
    type Root;

    fn open(
        root: &Self::Root,
        id: crate::file_store::StoreId,
        create: bool,
    ) -> Result<Self::Handle, crate::file_store::FileStoreError>;
}

#[cfg(all(feature = "server", unix))]
impl Open for crate::file_store::FsStore {
    type Root = std::path::PathBuf;

    fn open(
        root: &Self::Root,
        id: crate::file_store::StoreId,
        create: bool,
    ) -> Result<Self::Handle, crate::file_store::FileStoreError> {
        Self::open(root, id, create)
    }
}

#[cfg(all(test, feature = "server", not(target_arch = "wasm32")))]
impl Open for crate::file_store::MemoryStore {
    type Root = ();

    fn open(
        _: &Self::Root,
        id: crate::file_store::StoreId,
        create: bool,
    ) -> Result<Self::Handle, crate::file_store::FileStoreError> {
        Self::handles(id, create)
    }
}
