use std::{
    any::{type_name, Any, TypeId},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use quick_cache::sync::Cache;
use serde::{Deserialize, Serialize};
use sqlx::{
    query::Query,
    sqlite::{SqliteArguments, SqliteConnectOptions},
    Connection, Row, Sqlite, SqliteConnection,
};
use teloxide_core::types::{ChatId, UserId};
use tokio::sync::{Mutex, MutexGuard, OwnedMappedMutexGuard, OwnedMutexGuard};

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
    db: Mutex<Option<SqliteConnection>>,
}

impl DataStorage {
    pub async fn new() -> anyhow::Result<Self> {
        let mut db = SqliteConnection::connect_with(
            &SqliteConnectOptions::new()
                .filename("data.db")
                .create_if_missing(true),
        )
        .await?;
        sqlx::query(concat!(
            "create table if not exists data",
            "(ty text, user text, chat text, val blob, ",
            "primary key (ty, user, chat), unique (ty, user, chat))",
        ))
        .execute(&mut db)
        .await?;
        Ok(Self {
            cache: Cache::new(1000),
            db: Mutex::new(Some(db)),
        })
    }

    async fn get_db(&self) -> tokio::sync::MappedMutexGuard<'_, SqliteConnection> {
        MutexGuard::map(self.db.lock().await, |db| {
            db.as_mut().expect("Database connection is not initialized")
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
        let res = sqlx::query("select val from data where ty = $1 and user = $2 and chat = $3")
            .bind_id(type_name::<T>(), id)
            .fetch_optional(&mut *self.get_db().await)
            .await
            .expect("db read error")?;
        let res = T::deser_data(res.get::<&str, usize>(0));
        Some(Arc::new(Mutex::new(res)))
    }

    #[allow(dead_code)]
    pub async fn insert<T: DbData>(&'static self, id: DataId, val: T) {
        self.insert_raw(type_name::<T>(), id, &val.ser_data()).await;
        self.cache.insert(id, Arc::new(Mutex::new(val)));
    }
    async fn insert_raw(&'static self, ty: &str, id: DataId, val: &str) {
        sqlx::query(concat!(
            "insert into data(ty, user, chat, val) values ($1, $2, $3, $4) ",
            "on conflict(ty, user, chat) do update set val = $4"
        ))
        .bind_id(ty, id)
        .bind(val)
        .execute(&mut *self.get_db().await)
        .await
        .expect("db write error");
    }

    #[allow(dead_code)]
    pub async fn remove<T: DbData>(&'static self, id: DataId) {
        self.cache.remove(&id);
        sqlx::query("delete from data where ty = $1 and user = $2 and chat = $3")
            .bind_id(type_name::<T>(), id)
            .execute(&mut *self.get_db().await)
            .await
            .expect("db write error");
    }

    pub async fn close(&self) {
        let mut db = self.db.lock().await;
        if let Some(db) = db.take() {
            db.close()
                .await
                .expect("Failed to close database connection");
        }
    }
}

trait QueryExt<'q> {
    fn bind_id<'a>(self, ty: &'a str, id: DataId) -> Self
    where
        'a: 'q;
}
impl<'q> QueryExt<'q> for Query<'q, Sqlite, SqliteArguments> {
    fn bind_id<'a>(self, ty: &'a str, id: DataId) -> Self
    where
        'a: 'q,
    {
        self.bind(ty)
            .bind(ron::to_string(&id.user.map(|u| u.0)).expect("ser u64"))
            .bind(ron::to_string(&id.chat.map(|c| c.0)).expect("ser i64"))
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
