
#[derive(Clone)]
pub enum BranchSpec {
    ByBranchId(u32),
    ByBranchName(String),
}
impl <'a> BranchSpec {
    // for ingesting strings in the shape of "branch=name" or "branchId=2"
    pub fn parse(branch_spec_input: String) -> Result<BranchSpec, &'a str> {
        let branch_spec_string = branch_spec_input.to_string();
        if branch_spec_string.starts_with("branchId=") {
            let split_branch_id= branch_spec_string.split_once('=');
            match split_branch_id {
                Some(("branchId", id)) => {
                    let branch_id: u32 = id
                        .parse::<u32>()
                        .map_err(|_| "project-id should be a number")?;
                        Ok(BranchSpec::ByBranchId(branch_id))
                }
                _ => { Err("branch-id cannot be empty") }
            }
        } else if branch_spec_string.starts_with("branch=") {
            let split_branch_name= branch_spec_string.split_once('=');
            match split_branch_name {
                Some(("branch", name)) => Ok(BranchSpec::ByBranchName(name.to_string())),
                _ => { Err("branch-name cannot be empty") }
            }
        } else { Err("Must contain branch or branchId identifier.") }
    }
}

#[derive(Clone)]
pub struct ProjectContext {
    pub project_id: u32,
    pub branch_spec: Option<BranchSpec>,
    pub api_string: String,
}
impl <'a> ProjectContext {
    pub fn parse(project_context_input: &'a str) -> Result<ProjectContext, &'a str> {
        let input = project_context_input.to_string();
        if !input.contains(';') {
            let project_id = input.parse::<u32>().map_err(|_| "project id should be a number")?;
            Ok(ProjectContext { project_id, branch_spec: None, api_string: project_id.to_string() })
        }
        else {
            match input.split_once(';') {
                Some((project_id_input, branch_spec_string)) if project_id_input.chars().all(char::is_numeric) => {
                    let project_id = project_id_input.parse::<u32>().map_err(|_| "project id should be a number")?;
                    let branch_spec_result = BranchSpec::parse(branch_spec_string.to_string())?;
                    Ok(ProjectContext {
                        project_id,
                        branch_spec: Some(branch_spec_result.clone()),
                        api_string: project_context_input.to_string()
                    })
                },
                Some((_, "")) => Err("Must contain branch of branchId identifier."),
                Some(("", _)) => Err("Must contain a project ID."),
                Some((&_, &_)) | None => Err("Must contain a project ID.")
            }
        }
    }
}