use futures::TryStreamExt;
use mongodb::bson::{doc, Document};
pub use mongodb::options::{FindOneAndReplaceOptions, FindOptions};
use mongodb::results::{DeleteResult, InsertManyResult, InsertOneResult};
use mongodb::Collection;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use super::client::StoreClient;
use super::StoreError;
#[derive(Serialize, Deserialize, Debug)]
pub struct Pageable {
    pub page: i64,
    pub limit: i64,
    pub sort: Option<Document>,
}
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Page<T: Serialize + DeserializeOwned> {
    pub total_elements: i64,
    pub current_page: i64,
    pub next_page: Option<i64>,
    pub page_size: usize,
    pub content: Vec<T>,
}
#[derive(Debug, Clone)]
pub struct StoreRepository<T: Serialize + DeserializeOwned + Unpin + Send + Sync> {
    collection: Collection<T>,
    _db_name: String,
    _collection_name: String,
}

impl<T> StoreRepository<T>
where
    T: Serialize + DeserializeOwned + Unpin + Send + Sync,
{
    pub fn new(collection: Collection<T>, collection_name: &str, tenant_id: &str) -> Self {
        StoreRepository {
            collection,
            _db_name: tenant_id.to_string(),
            _collection_name: collection_name.to_string(),
        }
    }
}

impl<T> StoreRepository<T>
where
    T: Serialize + DeserializeOwned + Unpin + Send + Sync,
{
    pub fn get_repository(client: &StoreClient, collection_name: &str, tenant_id: &str) -> Self {
        let db = client.get_db(tenant_id);
        let collection = db.collection::<T>(collection_name);
        StoreRepository::new(collection, collection_name, tenant_id)
    }
    pub fn get_collection_name(&self) -> &str {
        &self._collection_name
    }
    pub fn get_db_name(&self) -> &str {
        &self._db_name
    }
}

impl<T> Repository<T> for StoreRepository<T>
where
    T: Serialize + DeserializeOwned + Unpin + Send + Sync + std::fmt::Debug,
{
    fn get_collection(&self) -> &Collection<T> {
        &self.collection
    }
}

#[async_trait::async_trait]
pub trait Repository<T: Serialize + DeserializeOwned + Unpin + Send + Sync + std::fmt::Debug> {
    fn get_collection(&self) -> &Collection<T>;

    async fn find_all(&self) -> Result<Vec<T>, StoreError> {
        let collection = self.get_collection();
        let cursor = collection
            .find(doc! {})
            .await
            .map_err(|e| StoreError { msg: e.to_string() })?;
        let collection: Vec<T> = cursor
            .try_collect()
            .await
            .map_err(|e| StoreError { msg: e.to_string() })?;
        Ok(collection)
    }

    async fn count(&self, query: Option<Document>) -> Result<u64, StoreError> {
        let collection = self.get_collection();
        let count = collection
            .count_documents(query.unwrap_or_else(|| doc! {}))
            .await
            .map_err(|e| StoreError { msg: e.to_string() })?;
        Ok(count)
    }

    async fn find_by_ids(&self, ids: Vec<String>) -> Result<Vec<T>, StoreError> {
        self.find_by_query(doc! {"_id": {"$in": ids}}, None).await
    }

    async fn find_by_query(
        &self,
        query: Document,
        options: impl Into<Option<FindOptions>> + Send,
    ) -> Result<Vec<T>, StoreError> {
        let collection = self.get_collection();
        let cursor = collection
            .find(query)
            .with_options(
                options
                    .into()
                    .unwrap_or_else(|| FindOptions::builder().build()),
            )
            .await
            .map_err(|e| StoreError { msg: e.to_string() })?;
        cursor
            .try_collect()
            .await
            .map_err(|e| StoreError { msg: e.to_string() })
    }
    async fn find_page_large_collection(
        &self,
        main_query: Option<Document>,
        last_element_id: Option<String>,
        limit: i64,
    ) -> Result<Vec<T>, StoreError> {
        let collection = self.get_collection();
        let query = {
            let cursor_query = if let Some(last_element_id) = last_element_id {
                doc! {
                    "_id": {
                        "$gt": last_element_id
                    }
                }
            } else {
                doc! {}
            };
            if let Some(main_query) = main_query {
                doc! {
                    "$and": [main_query, cursor_query]
                }
            } else {
                cursor_query
            }
        };

        let options = FindOptions::builder().limit(Some(limit)).build();
        let cursor = collection
            .find(query)
            .with_options(options)
            .await
            .map_err(|e| StoreError { msg: e.to_string() })?;
        let collection: Vec<T> = cursor
            .try_collect()
            .await
            .map_err(|e| StoreError { msg: e.to_string() })?;
        Ok(collection)
    }
    async fn find_page(
        &self,
        query: Option<Document>,
        pageable: Pageable,
    ) -> Result<Option<Page<T>>, StoreError> {
        let collection = self.get_collection();
        let count = self.count(query.clone()).await? as i64;
        let query = if let Some(q) = query {
            q
        } else {
            doc! {}
        };
        let skip = pageable.limit * pageable.page; // start at page 0
        if count <= skip {
            return Ok(Some(Page {
                total_elements: 0,
                current_page: 0,
                next_page: None,
                page_size: 0,
                content: vec![],
            }));
        }
        let options = FindOptions::builder()
            .skip(Some(skip as u64))
            .sort(pageable.sort)
            .limit(Some(pageable.limit))
            .build();
        let cursor = collection
            .find(query)
            .with_options(options)
            .await
            .map_err(|e| StoreError { msg: e.to_string() })?;
        let collection: Vec<T> = cursor
            .try_collect()
            .await
            .map_err(|e| StoreError { msg: e.to_string() })?;
        let next_page = if count > (pageable.limit * (pageable.page + 1)) {
            Some(pageable.page + 1)
        } else {
            None
        };
        let page_size = collection.len();
        let page = Page {
            total_elements: count,
            content: collection,
            current_page: pageable.page,
            next_page,
            page_size,
        };
        Ok(Some(page))
    }

    async fn delete_many(&self, query: Option<Document>) -> Result<DeleteResult, StoreError> {
        let query = if let Some(q) = query {
            q
        } else {
            doc! {}
        };
        let res = self
            .get_collection()
            .delete_many(query)
            .await
            .map_err(|e| StoreError { msg: e.to_string() })?;
        Ok(res)
    }

    async fn insert_many(&self, data: &Vec<T>) -> Result<InsertManyResult, StoreError> {
        let res = self
            .get_collection()
            .insert_many(data)
            .await
            .map_err(|e| StoreError { msg: e.to_string() })?;
        Ok(res)
    }

    async fn insert_one(&self, data: &T) -> Result<InsertOneResult, StoreError> {
        let res = self
            .get_collection()
            .insert_one(data)
            .await
            .map_err(|e| StoreError { msg: e.to_string() })?;
        Ok(res)
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<T>, StoreError> {
        let collection = self.get_collection();
        let res = collection
            .find_one(doc! {"_id": id})
            .await
            .map_err(|e| StoreError { msg: e.to_string() })?;
        Ok(res)
    }

    async fn find_one(&self, query: Option<Document>) -> Result<Option<T>, StoreError> {
        let collection = self.get_collection();
        let res = collection
            .find_one(query.unwrap_or_else(|| doc! {})) // it should always have a document, FIXME
            .await
            .map_err(|e| StoreError { msg: e.to_string() })?;
        Ok(res)
    }

    async fn delete_by_id(&self, id: &str) -> Result<Option<T>, StoreError> {
        self.delete_by_query(doc! {"_id": id}).await
    }

    async fn delete_by_query(&self, query: Document) -> Result<Option<T>, StoreError> {
        let collection = self.get_collection();
        let res = collection
            .find_one_and_delete(query)
            .await
            .map_err(|e| StoreError { msg: e.to_string() })?;
        Ok(res)
    }

    /// example:
    /// let filter = doc! { "targetUrl": "http://x.com", "status.type": "success" };
    /// let update = doc! { "$set": { "status.type": "archived" } };
    /// repository.update_many(filter, update).await?;
    async fn update_many(&self, filter: Document, replace: Document) -> Result<u64, StoreError> {
        let collection = self.get_collection();
        let update_result = collection
            .update_many(filter, replace)
            .await
            .map_err(|e| StoreError { msg: e.to_string() })?;
        Ok(update_result.modified_count)
    }

    async fn upsert(&self, id: &str, entity: &T) -> Result<Option<T>, StoreError> {
        let collection = self.get_collection();

        let options = FindOneAndReplaceOptions::builder()
            .upsert(Some(true))
            .build();
        let res = collection
            .find_one_and_replace(doc! {"_id": id}, entity)
            .with_options(options)
            .await
            .map_err(|e| StoreError { msg: e.to_string() })?;
        Ok(res)
    }
}
