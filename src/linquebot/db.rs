use std::{
    any::{type_name, Any, TypeId},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use duckdb::{Connection, ToSql};
use quick_cache::sync::Cache;
use serde::{Deserialize, Serialize};
use teloxide_core::types::{ChatId, UserId};
use tokio::sync::{Mutex, OwnedMappedMutexGuard, OwnedMutexGuard};

pub trait DbDataDyn: Any + Send + Sync {
    fn ser_data(&self) -> String;
}

pub trait DbData: DbDataDyn {
    fn deser_data(src: &str) -> Self;
}
impl<T: Any + Send + Sync + Serialize> DbDataDyn for T {
    fn ser_data(&self) -> String {
        ron::to_string(self).expect("ser error")
    }
}
impl<T: DbDataDyn + for<'a> Deserialize<'a>> DbData for T {
    fn deser_data(src: &str) -> Self {
        ron::from_str(src).expect("deser error")
    }
}

/// 数据库
///
/// 使用方式：
/// ```
/// ctx.db.of::<类型>().get_or_insert()
/// ctx.db.of::<类型>().chat(chat_id).get_or_insert()
/// ctx.db.of::<类型>().user(user_id).get_or_insert()
/// ctx.db.of::<类型>().chat(chat_id).user(user_id).get_or_insert()
/// ```
///
/// 参见 [crate::mods::markov]
#[derive(Debug)]
pub struct DataStorage {
    cache: Cache<DataId, Arc<Mutex<dyn DbDataDyn>>>,
    db: Mutex<Connection>,
}

impl DataStorage {
    pub async fn new() -> anyhow::Result<Self> {
        let db = Connection::open("data.duckdb")?;
        db.execute(
            concat!(
                "create table if not exists data",
                "(ty text, user text, chat text, val blob, ",
                "unique (ty, user, chat))",
            ),
            [],
        )?;

        Ok(Self {
            cache: Cache::new(1000),
            db: Mutex::new(db),
        })
    }

    pub fn of<T: DbData>(&'static self) -> DataBuilder<T> {
        DataBuilder {
            db: self,
            phantom_t: PhantomData,
            chat: None,
            user: None,
        }
    }

    pub async fn get<T: DbData>(&'static self, id: DataId) -> Option<DataGuard<T>> {
        let cache = if let Some(c) = self.cache.get(&id) {
            c
        } else {
            let res = self.get_from_db::<T>(id).await?;
            self.cache.insert(id, res.clone());
            res
        };
        Some(self.mk_insert_res::<T>(id, cache).await)
    }

    pub async fn get_or_insert<T: DbData>(
        &'static self,
        id: DataId,
        mk: impl FnOnce() -> T,
    ) -> DataGuard<T> {
        let cache = if let Some(c) = self.cache.get(&id) {
            c
        } else {
            let res = if let Some(r) = self.get_from_db::<T>(id).await {
                r
            } else {
                let r = mk();
                self.insert_raw(type_name::<T>(), id, &r.ser_data()).await;
                Arc::new(Mutex::new(r))
            };
            self.cache.insert(id, res.clone());
            res
        };
        self.mk_insert_res::<T>(id, cache).await
    }

    async fn mk_insert_res<T: DbData>(
        &'static self,
        id: DataId,
        cache: Arc<Mutex<dyn DbDataDyn>>,
    ) -> DataGuard<T> {
        let cache = cache.lock_owned().await;
        let Ok(sub) = OwnedMutexGuard::try_map(cache, |val| <dyn Any>::downcast_mut(val)) else {
            panic!(
                "Cached type mismatch: expected {:?}",
                std::any::type_name::<T>()
            );
        };
        DataGuard {
            db: self,
            id,
            changed: false,
            sub,
        }
    }

    async fn get_from_db<T: DbData>(&'static self, id: DataId) -> Option<Arc<Mutex<T>>> {
        let db = self.db.lock().await;
        let mut stmt = db
            .prepare("select val from data where ty = $1 and user = $2 and chat = $3")
            .expect("db read error");
        stmt.query_map(id.bind(type_name::<T>()), |row| {
            let res = row.get::<usize, String>(0)?;
            Ok(Arc::new(Mutex::new(T::deser_data(&res))))
        })
        .expect("db read error")
        .map(|i| i.expect("db read error"))
        .next()
    }

    #[allow(dead_code)]
    pub async fn insert<T: DbData>(&'static self, id: DataId, val: T) {
        self.insert_raw(type_name::<T>(), id, &val.ser_data()).await;
        self.cache.insert(id, Arc::new(Mutex::new(val)));
    }
    async fn insert_raw(&'static self, ty: &str, id: DataId, val: &str) {
        let db = self.db.lock().await;
        db.execute(
            concat!(
                "insert into data(ty, user, chat, val) values ($1, $2, $3, encode($4)) ",
                "on conflict(ty, user, chat) do update set val = encode($4)"
            ),
            id.bind_val(ty, val),
        )
        .expect("db write error");
    }

    #[allow(dead_code)]
    pub async fn remove<T: DbData>(&'static self, id: DataId) {
        self.cache.remove(&id);
        let db = self.db.lock().await;
        db.execute(
            "delete from data where ty = $1 and user = $2 and chat = $3",
            id.bind(type_name::<T>()),
        )
        .expect("db write error");
    }
}

pub struct DataGuard<T: DbData> {
    db: &'static DataStorage,
    id: DataId,
    changed: bool,
    sub: OwnedMappedMutexGuard<dyn DbDataDyn, T>,
}

impl<T: DbData> Deref for DataGuard<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.sub
    }
}
impl<T: DbData> DerefMut for DataGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.changed = true;
        &mut self.sub
    }
}
impl<T: DbData> Drop for DataGuard<T> {
    fn drop(&mut self) {
        let Self {
            db, id, changed, ..
        } = *self;
        let sub = self.sub.ser_data();
        tokio::spawn(async move {
            if changed {
                db.insert_raw(type_name::<T>(), id, &sub).await;
            }
        });
    }
}

pub struct DataBuilder<T: DbData> {
    db: &'static DataStorage,
    phantom_t: PhantomData<T>,
    chat: Option<ChatId>,
    user: Option<UserId>,
}

impl<T: DbData> DataBuilder<T> {
    pub fn chat(mut self, chat: ChatId) -> Self {
        self.chat = Some(chat);
        self
    }
    pub fn user(mut self, user: UserId) -> Self {
        self.user = Some(user);
        self
    }

    pub fn data_id(&self) -> DataId {
        DataId {
            ty: TypeId::of::<T>(),
            chat: self.chat,
            user: self.user,
        }
    }

    pub async fn get(self) -> Option<DataGuard<T>> {
        self.db.get(self.data_id()).await
    }

    #[allow(dead_code)]
    pub async fn insert(self, val: T) {
        self.db.insert(self.data_id(), val).await
    }

    pub async fn get_or_insert(self, mk: impl FnOnce() -> T) -> DataGuard<T> {
        self.db.get_or_insert(self.data_id(), mk).await
    }

    #[allow(dead_code)]
    pub async fn remove(self) {
        self.db.remove::<T>(self.data_id()).await
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct DataId {
    pub ty: TypeId,
    pub chat: Option<ChatId>,
    pub user: Option<UserId>,
}

impl DataId {
    pub fn bind(&self, ty: &str) -> [Box<dyn ToSql>; 3] {
        let ty = Box::new(ty.to_owned());
        let user = Box::new(self.user.map(|u| u.0));
        let chat = Box::new(self.chat.map(|c| c.0));
        [ty, user, chat]
    }

    pub fn bind_val(&self, ty: &str, val: &str) -> [Box<dyn ToSql>; 4] {
        let ty = Box::new(ty.to_owned());
        let user = Box::new(self.user.map(|u| u.0));
        let chat = Box::new(self.chat.map(|c| c.0));
        let val = Box::new(val.to_owned());
        [ty, user, chat, val]
    }
}
