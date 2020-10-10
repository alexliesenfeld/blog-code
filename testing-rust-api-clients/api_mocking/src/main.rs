use crate::GithubError::{MissingValueError, UnexpectedResponseCodeError};
use custom_error::custom_error;
use isahc::prelude::Request;
use isahc::{RequestExt, ResponseExt};
use serde_json::Value;cd

custom_error! {pub GithubError
    HttpError{source: isahc::http::Error} = "HTTP error",
    HttpClientError{source: isahc::Error} = "HTTP client error",
    ParserError{source: serde_json::Error} = "JSON parser error",
    UnexpectedResponseCodeError{code: u16} = "Unexpected HTTP response code: {code}",
    MissingValueError{field: &'static str} = "Missing field in HTTP response: {field}",
}

pub struct GithubAPIClient {
    base_url: String,
    token: String,
}

impl GithubAPIClient {
    pub fn new(token: String, base_url: String) -> GithubAPIClient {
        GithubAPIClient { base_url, token }
    }

    /// This method implements
    /// https://docs.github.com/en/rest/reference/repos#create-a-repository-for-the-authenticated-user
    pub fn create_repo(&self, name: &str, private: bool) -> Result<String, GithubError> {
        let mut response = Request::post(format!("{}/user/repos", self.base_url))
            .header("Authorization", format!("token {}", self.token))
            .header("Content-Type", "application/json")
            .body(
                json!({
                    "name": name,
                    "private": private
                })
                    .to_string(),
            )?
            .send()?;

        if response.status() != 201 {
            return Err(UnexpectedResponseCodeError { code: response.status().as_u16() });
        }

        let json_body: Value = response.json()?;
        return match json_body["html_url"].as_str() {
            Some(url) => Ok(url.into()),
            None => Err(MissingValueError { field: "html_url" }),
        };
    }
}

fn main() {
    let github = GithubAPIClient::new("<github-token>".into(), "https://api.github.com".into());
    let url = github
        .create_repo("apprepo", true)
        .expect("Cannot create repo");
    println!("Repo URL: {}", url);
}

#[cfg(test)]
mod tests {
    use crate::GithubAPIClient;
    use httpmock::{MockServer};
    use serde_json::json;

    #[test]
    fn create_repo_success_test() {
        let _ = env_logger::try_init();

        // Arrange
        let mock_server = MockServer::start();

        let mock = mock_server.mock(|when, then| {
            when.method("POST")
                .path("/user/repos")
                .header("Authorization", "token TOKEN")
                .header("Content-Type", "application/json");
            then.status(201)
                .json_body(json!({ "html_url": "http://example.com" }));
        });

        let github_client = GithubAPIClient::new("TOKEN".into(), mock_server.base_url());

        // Act
        let result = github_client.create_repo("testrepo", true);

        // Assert
        mock.assert();
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), "http://example.com");
    }
}
