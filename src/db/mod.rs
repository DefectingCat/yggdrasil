#[cfg(feature = "server")]
pub mod pool;

#[cfg(not(feature = "server"))]
#[allow(dead_code)]
pub mod pool {
    pub struct DummyPool;
    impl DummyPool {
        pub async fn get(&self) -> Result<(), ()> {
            Err(())
        }
    }
    pub static DB_POOL: DummyPool = DummyPool;

    pub async fn get_conn() -> Result<(), ()> {
        Err(())
    }
}
