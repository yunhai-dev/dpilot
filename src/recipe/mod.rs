use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum InputType {
    Text,
    Number,
    Password,
    Confirm,
    Select,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputSpec {
    pub id: String,
    #[serde(rename = "type")]
    pub input_type: InputType,
    pub prompt: String,
    #[serde(default)]
    pub default: Option<serde_yaml::Value>,
    #[serde(default)]
    pub options: Option<Vec<String>>,
    #[serde(default)]
    pub required: Option<bool>,
    #[serde(default)]
    pub help: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepSpec {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub working_dir: Option<String>,
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
    #[serde(default = "default_allow_failure")]
    pub allow_failure: Option<bool>,
    #[serde(default = "default_show_output")]
    pub show_output: Option<bool>,
}

fn default_allow_failure() -> Option<bool> {
    Some(false)
}

fn default_show_output() -> Option<bool> {
    Some(true)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    pub description: String,
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub inputs: Vec<InputSpec>,
    pub steps: Vec<StepSpec>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

fn default_category() -> String {
    "service".to_string()
}

impl Recipe {
    pub fn from_yaml_str(yaml_str: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml_str)
    }
}

pub mod registry;
