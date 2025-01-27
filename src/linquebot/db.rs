use std::{
    any::{type_name, Any, TypeId},
    future::{AsyncDrop, Future},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use quick_cache::sync::Cache;
use sqlx::{Connection, Row, SqliteConnection};
use teloxide_core::types::{ChatId, UserId};
use tokio::sync::{Mutex, OwnedMappedMutexGuard, OwnedMutexGuard};

pub trait DbData: Any + Send + Sync {
    fn persistent() -> bool
    where
        Self: Sized;
    fn from_str(src: &str) -> Self
    where
        Self: Sized;
    fn to_string(&self) -> String;
}

#[derive(Debug)]
pub struct DataStorage {
    cache: Cache<DataId, Arc<Mutex<dyn DbData>>>,
    db: Mutex<SqliteConnection>,
}

impl DataStorage {
    pub async fn new() -> anyhow::Result<Self> {
        Ok(Self {
            cache: Cache::new(1000),
            db: Mutex::new(SqliteConnection::connect("data.db").await?),
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
        assert_eq!(
            id.ty,
            TypeId::of::<T>(),
            "The type in DataId should be same as the type"
        );
        let cache = if let Some(c) = self.cache.get(&id) {
            c
        } else if T::persistent() {
            let res = sqlx::query("select json from data where ty = ? and user = ? and chat = ?")
                .bind(std::any::type_name::<T>())
                .bind(id.user.map(|u| u.0 as i64))
                .bind(id.chat.map(|c| c.0))
                .fetch_optional(&mut *self.db.lock().await)
                .await
                .expect("db read error")?;
            let res = T::from_str(res.get::<&str, usize>(0));
            let res = Arc::new(Mutex::new(res));
            self.cache.insert(id, res.clone());
            res
        } else {
            None?
        };
        let cache = cache.lock_owned().await;
        let Ok(sub) = OwnedMutexGuard::try_map(cache, |val| <dyn Any>::downcast_mut(val)) else {
            panic!("Cached type mismatch: expected {:?}", TypeId::of::<T>());
        };
        Some(DataGuard {
            db: self,
            id,
            changed: false,
            sub,
        })
    }

    pub async fn insert<T: DbData>(&'static self, id: DataId, val: T) {
        if T::persistent() {
            self.insert_raw(type_name::<T>(), id, &val.to_string()).await;
        }
        self.cache.insert(id, Arc::new(Mutex::new(val)));
    }
    async fn insert_raw(&'static self, ty: &str, id: DataId, val: &str) {
        sqlx::query(concat!(
            "insert into data(ty, user, chat, json) values (?, ?, ?, ?) ",
            "on conflict(ty, user, chat) do update set json = ?"
        ))
        .bind(ty)
        .bind(id.user.map(|u| u.0 as i64))
        .bind(id.chat.map(|c| c.0))
        .bind(val)
        .bind(val)
        .execute(&mut *self.db.lock().await)
        .await
        .expect("db write error");
    }

    pub async fn remove<T: DbData>(&'static self, id: DataId) {
        self.cache.remove(&id);
        if T::persistent() {
            sqlx::query("delete from data where ty = ? and user = ? and chat = ?")
                .bind(type_name::<T>())
                .bind(id.user.map(|u| u.0 as i64))
                .bind(id.chat.map(|c| c.0))
                .execute(&mut *self.db.lock().await)
                .await
                .expect("db write error");
        }
    }
}

pub struct DataGuard<T: DbData> {
    db: &'static DataStorage,
    id: DataId,
    changed: bool,
    sub: OwnedMappedMutexGuard<dyn DbData, T>,
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
impl<T: DbData> AsyncDrop for DataGuard<T> {
    type Dropper<'a> = impl Future<Output = ()>;

    fn async_drop(self: std::pin::Pin<&mut Self>) -> Self::Dropper<'_> {
        async move {
            if self.changed {
                self.db
                    .insert_raw(type_name::<T>(), self.id, &self.sub.to_string())
                    .await;
            }
        }
    }
}

pub struct DataBuilder<T: DbData> {
    db: &'static DataStorage,
    phantom_t: PhantomData<T>,
    chat: Option<ChatId>,
    user: Option<UserId>,
}

impl<T: DbData> DataBuilder<T> {
    pub fn chat(&mut self, chat: ChatId) -> &mut Self {
        self.chat = Some(chat);
        self
    }
    pub fn user(&mut self, user: UserId) -> &mut Self {
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

    pub async fn insert(self, val: T) {
        self.db.insert(self.data_id(), val).await
    }

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
