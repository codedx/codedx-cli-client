/*
 * Copyright 2021 Code Dx, Inc
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use crate::branching::*;
use crate::config::ClientConfig;
use reqwest;
use::reqwest::Method;
use serde::de::DeserializeOwned;
use serde::ser::Serialize;
use serde_json;
use std;
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::Path;
use std::thread;
use std::time::Duration;


/// Project filter criteria used with `ApiClient::query_projects` to define project filter criteria.
#[derive(Debug, Serialize)]
pub struct ApiProjectFilter<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<&'a str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<&'a str, &'a str>>
}

/// A project provided by the Code Dx API.
#[derive(Debug, Deserialize, Serialize)]
pub struct ApiProject {
    pub id: u32,
    pub name: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<u32>,
}

/// Branch filter criteria used with `ApiClient::query_branches` to define project filter criteria.
#[derive(Debug, Serialize)]
pub struct ApiBranchFilter<'a> {
    pub project_id: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<&'a str>
}

/// A branch provided by the Code Dx API.
#[derive(Debug, Deserialize, Serialize)]
pub struct ApiBranch {
    pub id: u32,
    pub name: String,
    #[serde(rename = "projectId")]
    pub project_id: u32,
    #[serde(rename = "isDefault")]
    pub is_default: bool,
}

/// The response the server gives when you successfully start an analysis via the "stable" start-analysis endpoint.
#[derive(Debug, Deserialize)]
pub struct ApiAnalysisJobResponse {
    #[serde(rename = "analysisId")]
    pub analysis_id: u32,
    #[serde(rename = "jobId")]
    pub job_id: String
}

/// The response the server gives when you successfully start an analysis via the "stable" start-analysis endpoint.
#[derive(Debug, Deserialize)]
pub struct ApiAnalysisWithGitSourceJobResponse {
    #[serde(rename = "jobId")]
    pub job_id: String
}

/// Enumeration representing the 5 possible statuses a Code Dx "job" may be in.
#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Running,
    Cancelled,
    Completed,
    Failed
}
impl JobStatus {
    pub fn is_ready(&self) -> bool {
        match *self {
            JobStatus::Completed => true,
            JobStatus::Failed => true,
            _ => false
        }
    }
    #[allow(dead_code)]
    pub fn is_success(&self) -> bool {
        match *self {
            JobStatus::Completed => true,
            _ => false
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JobStatusResponse {
    /// ID of the requested job.
    ///
    /// This should be the same as the `job_id` sent when you requested the status in the first place.
    #[serde(rename = "jobId")]
    pub job_id: String,

    /// The actual job status.
    pub status: JobStatus,

    // there are some optional fields like "progress", "blockedBy", and "reason"
    // which are present depending on the status, but they aren't necessary for
    // our use case, so I'm not going to model them.
}

/// Things that can go wrong when making requests with the API.
#[derive(Debug)]
pub enum ApiError {
    /// Covers communications errors. Problems with HTTPS (typically cert issues), problems with IO,
    /// problems where the server responded with JSON that this client doesn't know how to parse, etc.
    Protocol(reqwest::Error),

    /// Generated by `ApiClient::expect_success` when the response code was not 2xx.
    ///
    /// Additionally holds the error response, which will be an `ApiErrorMessage::Nice`
    /// for most expected error cases, but may sometimes be an `ApiErrorMessage::Raw`,
    /// typically for 5xx internal error responses.
    NonSuccess(reqwest::StatusCode, ApiErrorMessage),

    /// Covers some I/O error cases like when the server's response body couldn't be read to a String,
    /// and when a file couldn't be added to a multipart form body.
    IO(std::io::Error),
}
impl From<std::io::Error> for ApiError {
    fn from(e: std::io::Error) -> ApiError {
        ApiError::IO(e)
    }
}
impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> ApiError {
        ApiError::Protocol(err)
    }
}

#[derive(Debug)]
pub enum ApiErrorMessage {
    Nice(String),
    Raw(String)
}
impl ApiErrorMessage {
    fn from_body(response: reqwest::blocking::Response) -> Result<ApiErrorMessage, ApiError> {
        response.text().map_err(ApiError::from).and_then(|body|{
            serde_json::from_str::<ErrorMessageResponse>(&body)
                .map(|err_body| ApiErrorMessage::Nice(err_body.error))
                .or_else(|_| Ok(ApiErrorMessage::Raw(body)))
        })
    }
}

/// Represents the usual structure of error messages generated by Code Dx for expected errors.
///
/// I.e. for response status codes 4xx the server will typically respond with `{ "error": "some error message" }`.
/// This struct exists so that such responses can be deserialized and converted to `ApiErrorMessage`
/// instances.
#[derive(Deserialize)]
struct ErrorMessageResponse {
    error: String
}

/// Defines a polling strategy based on the iteration number and current state of the poll.
///
/// The `next_wait` function decides how long the polling process should wait before re-checking the state.
/// If it returns `Some(duration)`, the polling process will wait that duration before re-checking.
/// If it returns `None`, the polling process will immediately end, typically returning the latest state.
///
/// The `iteration_number` will start at `1` and increment every time `next_wait` is called for the current poll.
pub trait PollingStrategy<T> {
    fn next_wait(&self, iteration_number: usize, state: &T) -> Option<Duration>;
}

/// Simple polling strategy that always waits a fixed amount of time between iterations.
impl <T: Debug> PollingStrategy<T> for Duration {
    fn next_wait(&self, iteration_number: usize, state: &T) -> Option<Duration> {
        println!("# Polling job completion, iteration {}: status = {:?}", iteration_number, state);
        Some(*self)
    }
}

pub type ApiResult<T> = Result<T, ApiError>;


/// Wrapper for results coming out of the `ApiClient`.
///
/// Enables a "chained" way of reacting to API responses, e.g.
///
/// ```
/// let result: ApiResult<Vec<ApiProject>> = api_response
///     .expect_success()
///     .expect_json();
/// ```
pub struct ApiResponse(ApiResult<reqwest::blocking::Response>);
impl ApiResponse {
    pub fn from(r: ApiResult<reqwest::blocking::Response>) -> ApiResponse {
        ApiResponse(r)
    }

    pub fn get(self) -> ApiResult<reqwest::blocking::Response> {
        self.0
    }

    pub fn expect_success(self) -> ApiResponse {
        ApiResponse(self.0.and_then(move |response| {
            let status = response.status();
            if status.is_success() {
                Ok(response)
            } else {
                ApiErrorMessage::from_body(response).and_then(|response_msg| {
                    Err(ApiError::NonSuccess(status, response_msg))
                })
            }
        }))
    }

    pub fn expect_json<T: DeserializeOwned>(self) -> ApiResult<T> {
        self.0.and_then(|response| {
            response.json().map_err(ApiError::from)
        })
    }
}

/// Main entry point for interacting with the Code Dx REST API.
pub struct ApiClient {
    config: Box<ClientConfig>,
    client: reqwest::blocking::Client
}

impl ApiClient {
    pub fn new(config: Box<ClientConfig>) -> ApiClient {
        let client_builder = reqwest::blocking::Client::builder();
        // the --insecure CLI flag enables this, to disable TLS hostname verification
        let client_builder = if config.allows_insecure() {
            client_builder.danger_accept_invalid_hostnames(true)
        } else {
            client_builder
        };
        let client = client_builder.build().unwrap();
        ApiClient { config, client }
    }

    pub fn get_config(&self) -> &ClientConfig {
        self.config.as_ref()
    }

    pub fn get_job_status(&self, job_id: &str) -> ApiResult<JobStatus> {
        self.api_get(&["api", "jobs", job_id])
            .expect_success()
            .expect_json::<JobStatusResponse>()
            .map(|jsr| jsr.status)
    }

    /// Repeatedly call `get_job_status(job_id)` until it returns an error or a "ready" status.
    ///
    /// Uses the provided `polling_stategy` to determine how long to wait between each status
    /// check, and whether to abort early.
    ///
    /// If the `polling_strategy` decides to abort early, the result of the poll will be the
    /// most recent `JobStatus` to be passed.
    ///
    /// If at any point the job status check fails (i.e. `get_job_status` returns an `Err(_)`),
    /// the poll will immediately stop, returning that error.
    pub fn poll_job_completion<P: PollingStrategy<JobStatus>>(&self, job_id: &str, polling_strategy: P) -> ApiResult<JobStatus> {
        let mut iteration_number: usize = 0;
        loop {
            let status_result = self.get_job_status(job_id);
            iteration_number += 1;
            match status_result {
                Ok(status) => {
                    if status.is_ready() {
                        break status_result;
                    } else {
                        // call the "step" function to see if the poll should continue,
                        // and if so, how long it should wait before checking again
                        match polling_strategy.next_wait(iteration_number, &status) {
                            Some(wait_dur) => thread::sleep(wait_dur),
                            None => break status_result,
                        }
                    }
                },
                Err(_) => break status_result,
            }
        }
    }

    pub fn get_job_result(&self, job_id: &str) -> ApiResult<ApiAnalysisJobResponse> {
        self.api_get(&["api", "jobs", job_id, "result"])
            .expect_success()
            .expect_json()

    }

    pub fn get_projects(&self) -> ApiResult<Vec<ApiProject>> {
        self.api_get(&["x", "projects"])
            .expect_success()
            .expect_json()
    }

    pub fn query_projects<'a>(&self, filter: &'a ApiProjectFilter) -> ApiResult<Vec<ApiProject>> {
        self.api_post(&["x", "projects", "query"], json!({ "filter": filter }))
            .expect_success()
            .expect_json()
    }

    pub fn get_branches_for_project(&self, project_id: u32) -> ApiResult<Vec<ApiBranch>> {
        self.api_get(&["x", "projects", &project_id.to_string(), "branches"])
            .expect_success()
            .expect_json()
    }

    pub fn query_branches_for_project(&self, project_id: u32, branch_name: &str) -> ApiResult<Vec<ApiBranch>> {
        let branch_name_lowercase = branch_name.to_lowercase();
        self.get_branches_for_project(project_id).map(|branches| {
            branches
                .into_iter()
                .filter(|branch| branch.name.to_lowercase().contains(&branch_name_lowercase))
                .collect()
        })
    }

    pub fn start_analysis(&self, project_context: ProjectContext, branch_name: Option<String>, files: Vec<&Path>) -> ApiResult<ApiAnalysisJobResponse> {
        let branch_name_string = branch_name.unwrap_or_default();
        let form = files
            .iter()
            .enumerate()
            .fold(Ok(reqwest::blocking::multipart::Form::new()),  move |maybe_form, (index, file)| {
                maybe_form.and_then(|mut form| {
                    if !branch_name_string.is_empty() {
                        form = form.text("branchName", branch_name_string.clone())
                    }
                    form.file(format!("file{}", index), file)
                })
            })
            .map_err(ApiError::from);

        form.and_then(|form| {
            self.api_post(&["api", "projects", &project_context.api_string, "analysis"], form)
                .expect_success()
                .expect_json::<ApiAnalysisJobResponse>()
        })
    }

    pub fn start_analysis_with_git(&self, project_context: ProjectContext, branch_name: Option<String>, include_git_source: bool, git_branch_name: Option<String>, files: Vec<&Path>) -> ApiResult<ApiAnalysisWithGitSourceJobResponse> {
        let branch_name_string = branch_name.unwrap_or_default();
        let git_branch_name_string = git_branch_name.unwrap_or_default();

        let form = files
            .iter()
            .enumerate()
            .fold(Ok(reqwest::blocking::multipart::Form::new()), |maybe_form, (index, file)| {
                maybe_form.and_then(|mut form| {
                    if !branch_name_string.is_empty() {
                        form = form.text("branchName", branch_name_string.clone())
                    }
                    if !git_branch_name_string.is_empty() {
                        form = form.text("gitBranchName", git_branch_name_string.clone())
                    }
                    form = form.text("includeGitSource", include_git_source.to_string());
                    form.file(format!("file{}", index), file)
                })
            })
            .map_err(ApiError::from);

        form.and_then(|form| {
            self.api_post(&["api", "projects", &project_context.project_id.to_string(), "analysis"], form)
                .expect_success()
                .expect_json::<ApiAnalysisWithGitSourceJobResponse>()
        })
    }

    pub fn set_analysis_name(&self, project_context: ProjectContext, analysis_id: u32, name: &str) -> ApiResult<()> {
        self.api_put(&["x", "projects", &project_context.project_id.to_string(), "analyses", &analysis_id.to_string()], json!({ "name": name }))
            .expect_success()
            .get()
            .map(|_| ())
    }

    pub fn api_get(&self, path_segments: &[&str]) -> ApiResponse {
        self.api_request(Method::GET, path_segments, ReqBody::None)
    }

    pub fn api_post<B>(&self, path_segments: &[&str], body: B) -> ApiResponse
        where B: Into<ReqBody>
    {
        self.api_request(Method::POST, path_segments, body)
    }

    pub fn api_put<B>(&self, path_segments: &[&str], body: B) -> ApiResponse
        where B: Into<ReqBody>
    {
        self.api_request(Method::PUT, path_segments, body)
    }

    pub fn api_request<B>(&self, method: Method, path_segments: &[&str], body: B) -> ApiResponse
        where B: Into<ReqBody>
    {
        let url = self.config.api_url(path_segments);
        let request_builder = self.client.request(method, url);
        let configured_rb = self.config.apply_auth(request_builder);
        match body.into() {
            ReqBody::Json(ref json) => {
                ApiResponse::from(configured_rb.json(json).send().map_err(ApiError::from))
            },
            ReqBody::Form(form) => {
                ApiResponse::from(configured_rb.multipart(form).send().map_err(ApiError::from))
            },
            ReqBody::None => ApiResponse::from(configured_rb.send().map_err(ApiError::from))
        }
    }
}

/// Collection of types that `ApiClient` knows how to use as a request body.
pub enum ReqBody {
    /// A multipart form, typically used for file uploads.
    Form(reqwest::blocking::multipart::Form),
    /// A JSON object as the body
    Json(serde_json::Value),
    /// No body
    None,
}
impl ReqBody {
    #[allow(dead_code)]
    pub fn as_json<T: Serialize>(body: T) -> ReqBody {
        ReqBody::Json(serde_json::to_value(body).unwrap())
    }
}
impl From<serde_json::Value> for ReqBody {
    fn from(json: serde_json::Value) -> ReqBody {
        ReqBody::Json(json)
    }
}
impl From<reqwest::blocking::multipart::Form> for ReqBody {
    fn from(form: reqwest::blocking::multipart::Form) -> ReqBody {
        ReqBody::Form(form)
    }
}