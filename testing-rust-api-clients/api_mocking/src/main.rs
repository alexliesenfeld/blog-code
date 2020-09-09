use custom_error::custom_error;
use isahc::prelude::Request;
use serde_json::{json, Value};
use isahc::{RequestExt, ResponseExt};
use crate::GithubError::{MissingValueError, UnexpectedResponseCodeError};

custom_error! {pub GithubError
    HttpError{source: isahc::http::Error} = "HTTP error",
    HttpClientError{source: isahc::Error} = "HTTP client error",
    ParserError{source: serde_json::Error} = "JSON parser error",
    UnexpectedResponseCodeError{code: u16} = "Unexpected HTTP response code: {code}",
    MissingValueError{field: &'static str} = "Missing field in HTTP response: {field}",
}

pub struct GithubAPIAdapter {
    base_url: String,
    token: String,
}

impl GithubAPIAdapter {
    pub fn new(token: String, base_url: String) -> GithubAPIAdapter {
        GithubAPIAdapter {
            base_url,
            token,
        }
    }

    /// This method implements
    /// https://docs.github.com/en/rest/reference/repos#create-a-repository-for-the-authenticated-user
    pub fn create_repo(&self, name: &str, private: bool) -> Result<String, GithubError> {
        let mut response = Request::post(format!("{}/user/repos", self.base_url))
            .header("Authorization", format!("token {}", self.token))
            .header("Content-Type", "application/json")
            .body(json!({
                    "name": name,
                    "private": private
                }).to_string())?
            .send()?;

        if response.status() != 201 {
            return Err(UnexpectedResponseCodeError { code: response.status().as_u16() });
        }

        let json_body: Value = response.json()?;
        return match json_body["html_url"].as_str() {
            Some(url) => Ok(url.into()),
            None => Err(MissingValueError{field: "html_url"})
        }
    }
}

fn main() {
    let github = GithubAPIAdapter::new("<github-token>".into(), "https://api.github.com".into());
    let url = github
        .create_repo("apprepo", true)
        .expect("Cannot create repo");
    println!("Repo URL: {}", url);
}

#[cfg(test)]
mod tests {
    use crate::GithubAPIAdapter;
    use httpmock::{MockServer, Mock, Method::POST};
    use serde_json::{json};

    #[test]
    fn create_repo_success_test() {
        let _ = env_logger::try_init();

        // Arrange
        let mock_server = MockServer::start();
        let mock = Mock::new()
            .expect_method(POST)
            .expect_path("/user/repos")
            .expect_header("Authorization", "token TOKEN")
            .expect_header("Content-Type", "application/json")
            .return_status(201)
            .return_body(&json!({ "html_url": "http://example.com" }).to_string())
            .create_on(&mock_server);

        let adapter = GithubAPIAdapter::new("TOKEN".into(), format!("http://{}", &mock_server.address()));

        // Act
        let result = adapter
            .create_repo("testrepo", true)
            .expect("Request not successful");

        // Assert
        assert_eq!(result, "http://example.com");
        assert_eq!(mock.times_called(), 1);
    }
}

