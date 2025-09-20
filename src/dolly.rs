use crate::git;
use regex::Regex;

#[derive(PartialEq, Debug)]
pub struct GitRepo {
    pub host: String,
    pub slug: String,
    pub repo_name: String,
}
pub fn valid_ssh_url(url: &str) -> bool {
    let matches = Regex::new(r"(git)@([^/:]+):([^/:]+)/(.+)(.git)");

    match matches {
        Ok(content) => content.is_match(url),
        Err(_) => false,
    }
}

pub fn make_url(url: &str) -> String {
    make_url_private(parse_url(url))
}
pub fn make_url_private(git_repo: GitRepo) -> String {
    String::from(&format!(
        "https://{}/{}/{}",
        git_repo.host, git_repo.slug, git_repo.repo_name
    ))
}
pub fn parse_url(url: &str) -> GitRepo {
    let re = Regex::new(r"(git)@([^/:]+):(.+)/([^/:]+)(.git)").expect("failed to parse regex");

    let caps = re.captures(&url).unwrap();
    let host = caps.get(2).map_or("", |m| m.as_str());
    let slug = caps.get(3).map_or("", |m| m.as_str());
    let repo_name = caps.get(4).map_or("", |m| m.as_str());
    GitRepo {
        host: host.parse().unwrap(),
        slug: slug.parse().unwrap(),
        repo_name: repo_name.parse().unwrap(),
    }
}

pub fn project_to_repo(projects: Vec<git::Project>) -> Vec<GitRepo> {
    projects
        .iter()
        .map(|p| parse_url(&*p.ssh_url_to_repo))
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_works() {
        let projects = vec![git::Project {
            ssh_url_to_repo: "git@gitlab.company.dev:squad/tools/mytool.git".to_string(),
            path_with_namespace: "".to_string(),
        }];
        let result = project_to_repo(projects);
        assert_eq!(
            result[0],
            GitRepo {
                host: "gitlab.company.dev".to_string(),
                slug: "squad/tools".to_string(),
                repo_name: "mytool".to_string(),
            }
        );
    }
}
