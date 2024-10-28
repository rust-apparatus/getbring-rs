use anyhow::{Context, Result};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
use serde::Deserialize;
use std::collections::HashMap;

const API_BASE_URL: &str = "https://api.getbring.com/rest/v2/";
const API_KEY: &str = "cof4Nc6D8saplXjE3h3HXqHH8m7VU2i1Gs0g85Sp";

#[derive(Debug, Clone)]
pub struct BringClient {
    email: String,
    password: String,
    client: Client,
    base_url: String,
    uuid: Option<String>,
    bearer_token: Option<String>,
    refresh_token: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AuthResponse {
    pub name: String,
    pub uuid: String,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct GetItemsResponseEntry {
    pub specification: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct GetItemsResponse {
    pub uuid: String,
    pub status: String,
    pub purchase: Vec<GetItemsResponseEntry>,
    pub recently: Vec<GetItemsResponseEntry>,
}

#[derive(Debug, Deserialize)]
pub struct GetAllUsersFromListEntry {
    #[serde(rename = "publicUuid")]
    pub public_uuid: String,
    pub name: String,
    pub email: String,
    #[serde(rename = "photoPath")]
    pub photo_path: String,
    #[serde(rename = "pushEnabled")]
    pub push_enabled: bool,
    #[serde(rename = "plusTryOut")]
    pub plus_try_out: bool,
    pub country: String,
    pub language: String,
}

#[derive(Debug, Deserialize)]
pub struct GetAllUsersFromListResponse {
    pub users: Vec<GetAllUsersFromListEntry>,
}

#[derive(Debug, Deserialize)]
pub struct LoadListsEntry {
    #[serde(rename = "listUuid")]
    pub list_uuid: String,
    pub name: String,
    pub theme: String,
}

#[derive(Debug, Deserialize)]
pub struct LoadListsResponse {
    pub lists: Vec<LoadListsEntry>,
}

#[derive(Debug, Deserialize)]
pub struct GetItemsDetailsEntry {
    pub uuid: String,
    #[serde(rename = "itemId")]
    pub item_id: String,
    #[serde(rename = "listUuid")]
    pub list_uuid: String,
    #[serde(rename = "userIconItemId")]
    pub user_icon_item_id: String,
    #[serde(rename = "userSectionId")]
    pub user_section_id: String,
    #[serde(rename = "assignedTo")]
    pub assigned_to: String,
    #[serde(rename = "imageUrl")]
    pub image_url: String,
}

impl BringClient {
    pub fn new(email: String, password: String) -> Self {
        Self {
            email,
            password,
            client: Client::new(),
            base_url: API_BASE_URL.to_string(),
            uuid: None,
            bearer_token: None,
            refresh_token: None,
            name: None,
        }
    }

    fn get_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("X-BRING-API-KEY", HeaderValue::from_static(API_KEY));
        headers.insert("X-BRING-CLIENT", HeaderValue::from_static("webApp"));
        headers.insert("X-BRING-CLIENT-SOURCE", HeaderValue::from_static("webApp"));
        headers.insert("X-BRING-COUNTRY", HeaderValue::from_static("DE"));

        if let Some(uuid) = &self.uuid {
            headers.insert("X-BRING-USER-UUID", HeaderValue::from_str(uuid).unwrap());
        }

        if let Some(token) = &self.bearer_token {
            headers.insert(
                "Authorization",
                HeaderValue::from_str(&format!("Bearer {}", token)).unwrap(),
            );
        }

        headers
    }

    pub async fn login(&mut self) -> Result<()> {
        let params = [
            ("email", self.email.as_str()),
            ("password", self.password.as_str()),
        ];

        let response = self
            .client
            .post(format!("{}bringauth", self.base_url))
            .form(&params)
            .send()
            .await
            .context("Failed to send login request")?;

        let auth_response: AuthResponse = response
            .json()
            .await
            .context("Failed to parse login response")?;

        self.name = Some(auth_response.name);
        self.uuid = Some(auth_response.uuid);
        self.bearer_token = Some(auth_response.access_token);
        self.refresh_token = Some(auth_response.refresh_token);

        Ok(())
    }

    pub async fn load_lists(&self) -> Result<LoadListsResponse> {
        let uuid = self.uuid.as_ref().context("Not logged in")?;

        let response = self
            .client
            .get(format!("{}bringusers/{}/lists", self.base_url, uuid))
            .headers(self.get_headers())
            .send()
            .await
            .context("Failed to load lists")?;

        response
            .json()
            .await
            .context("Failed to parse lists response")
    }

    pub async fn get_items(&self, list_uuid: &str) -> Result<GetItemsResponse> {
        let response = self
            .client
            .get(format!("{}bringlists/{}", self.base_url, list_uuid))
            .headers(self.get_headers())
            .send()
            .await
            .context("Failed to get items")?;

        response
            .json()
            .await
            .context("Failed to parse items response")
    }

    pub async fn get_items_details(&self, list_uuid: &str) -> Result<Vec<GetItemsDetailsEntry>> {
        let response = self
            .client
            .get(format!("{}bringlists/{}/details", self.base_url, list_uuid))
            .headers(self.get_headers())
            .send()
            .await
            .context("Failed to get item details")?;

        response
            .json()
            .await
            .context("Failed to parse item details response")
    }

    pub async fn save_item(
        &self,
        list_uuid: &str,
        item_name: &str,
        specification: &str,
    ) -> Result<()> {
        let mut headers = self.get_headers();
        headers.insert(
            "Content-Type",
            HeaderValue::from_static("application/x-www-form-urlencoded; charset=UTF-8"),
        );

        let params = [
            ("purchase", item_name),
            ("recently", ""),
            ("specification", specification),
            ("remove", ""),
            ("sender", "null"),
        ];

        self.client
            .put(format!("{}bringlists/{}", self.base_url, list_uuid))
            .headers(headers)
            .form(&params)
            .send()
            .await
            .context("Failed to save item")?;

        Ok(())
    }

    pub async fn remove_item(&self, list_uuid: &str, item_name: &str) -> Result<()> {
        let mut headers = self.get_headers();
        headers.insert(
            "Content-Type",
            HeaderValue::from_static("application/x-www-form-urlencoded; charset=UTF-8"),
        );

        let params = [
            ("purchase", ""),
            ("recently", ""),
            ("specification", ""),
            ("remove", item_name),
            ("sender", "null"),
        ];

        self.client
            .put(format!("{}bringlists/{}", self.base_url, list_uuid))
            .headers(headers)
            .form(&params)
            .send()
            .await
            .context("Failed to remove item")?;

        Ok(())
    }

    pub async fn get_all_users_from_list(
        &self,
        list_uuid: &str,
    ) -> Result<GetAllUsersFromListResponse> {
        let response = self
            .client
            .get(format!("{}bringlists/{}/users", self.base_url, list_uuid))
            .headers(self.get_headers())
            .send()
            .await
            .context("Failed to get users from list")?;

        response
            .json()
            .await
            .context("Failed to parse users response")
    }

    pub async fn load_translations(&self, locale: &str) -> Result<HashMap<String, String>> {
        let response = self
            .client
            .get(format!(
                "https://web.getbring.com/locale/articles.{}.json",
                locale
            ))
            .send()
            .await
            .context("Failed to load translations")?;

        response
            .json()
            .await
            .context("Failed to parse translations")
    }

    /// Gets a list UUID by its name. Returns None if no list with the given name is found.
    /// Case-insensitive matching is used for convenience.
    ///
    /// # Arguments
    /// * `list_name` - The name of the list to find
    ///
    /// # Returns
    /// * `Result<Option<String>>` - The UUID of the list if found, None otherwise
    ///
    /// # Example
    /// ```rust
    /// let list_id = client.get_list_id_by_name("Groceries").await?;
    /// if let Some(id) = list_id {
    ///     println!("Found list ID: {}", id);
    /// } else {
    ///     println!("List not found!");
    /// }
    /// ```
    pub async fn get_list_id_by_name(&self, list_name: &str) -> Result<Option<String>> {
        let lists = self.load_lists().await?;

        // Case-insensitive search for the list name
        Ok(lists
            .lists
            .into_iter()
            .find(|list| list.name.to_lowercase() == list_name.to_lowercase())
            .map(|list| list.list_uuid))
    }

    /// Gets a list UUID by its name, returning an error if the list is not found.
    /// This is a convenience wrapper around get_list_id_by_name for cases where
    /// you expect the list to exist.
    ///
    /// # Arguments
    /// * `list_name` - The name of the list to find
    ///
    /// # Returns
    /// * `Result<String>` - The UUID of the list
    ///
    /// # Errors
    /// Returns an error if the list is not found or if there's an API error
    pub async fn get_list_id_by_name_required(&self, list_name: &str) -> Result<String> {
        self.get_list_id_by_name(list_name)
            .await?
            .context(format!("List with name '{}' not found", list_name))
    }
}
