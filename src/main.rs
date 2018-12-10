extern crate mktemp;
extern crate reqwest;
extern crate serde;
extern crate serde_json;

use mktemp::Temp;
use std::{fs, fs::File, fs::OpenOptions, io, io::Error, io::ErrorKind, io::Write, path::Path};

#[macro_use]
extern crate serde_derive;

#[derive(Deserialize)]
struct Assignee {
    displayName: String,
}

#[derive(Deserialize)]
struct IssueType {
    name: String,
}

#[derive(Deserialize)]
struct Fields {
    assignee: Assignee,
    issuetype: IssueType,
    summary: String,
}

#[derive(Deserialize)]
struct Issue {
    fields: Fields,
}

fn main() -> io::Result<()> {
    // git log --oneline 1.0.5...1.0.4 --no-merges | grep -ioE "issues?:\s#\w+-\w+\S" | sort -u

    let username = get_from_input("your Jira username".to_string())?;
    let password = get_from_input("your Jira password".to_string())?;
    let old_version = get_from_input("the current tag".to_string())?;
    let new_version = get_from_input("the new tag".to_string())?;

    // @todo - get the issues from the git commit log, not just hard-coded
    let issues = vec!["SSSVCS-4849", "SSSVCS-4850", "SSSVCS-4989"];
    let client = reqwest::Client::new();
    let jira_url = "https://jira.pgi-tools.com";
    let base_url = jira_url.to_owned() + "/rest/api/2/issue/";

    println!("Generating CHANGES updates for {}", new_version);

    let changes_entries: Vec<String> = issues
        .iter()
        .map(|issue| {
            print!(
                "Requesting data for issue: {api_url}...",
                api_url = (base_url.to_owned() + issue).as_str()
            );
            let res = client
                .get((base_url.to_owned() + issue).as_str())
                .basic_auth(username.clone(), Some(password.clone()))
                .send();
            res.map(|mut response| {
                let json: Issue = response.json().expect("Unable to parse json from Jira");
                println!("Done");
                return output_issue(
                    json,
                    format!("{jira}/browse/{issue}", jira = jira_url, issue = issue),
                );
            }).expect("Failed to request info for issue")
        }).collect();

    let full_entry = format!(
        "# {version}\n{changes}\n",
        version = new_version,
        changes = changes_entries.join("\n")
    );
    println!("Writing to changes md: {}", full_entry);
    let changes_path = Path::new("CHANGES.md");
    prepend_file(full_entry.as_bytes(), &changes_path)?;

    Ok(())
}

fn get_from_input(input_name: String) -> Result<String, Error> {
    let mut input_str = String::new();

    println!("Enter {}:", input_name);
    io::stdin()
        .read_line(&mut input_str)
        .expect(format!("Failed to get {}", input_name).as_str());

    if input_str.trim().len() < 1 {
        Err(Error::new(
            ErrorKind::InvalidInput,
            format!("{} is invalid", input_name),
        ))
    } else {
        Ok(input_str.trim().to_string())
    }
}

fn output_issue(issue: Issue, issue_url: String) -> String {
    let type_str = if issue.fields.issuetype.name == "Story" {
        "Improvement"
    } else {
        "Resolved"
    };

    format!(
        "- {type}: {summary}, by {author}. [See on Jira]({url})",
        type = type_str, summary = issue.fields.summary, author = issue.fields.assignee.displayName, url = issue_url
    )
}

fn prepend_file<P: AsRef<Path>>(data: &[u8], file_path: &P) -> io::Result<()> {
    // Create a temporary file
    let mut tmp_path = Temp::new_file()?;
    // Stop the temp file being automatically deleted when the variable
    // is dropped, by releasing it.
    tmp_path.release();
    // Open temp file for writing
    let mut tmp = File::create(&tmp_path)?;
    // Open source file for reading
    let mut src = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&file_path)?;

    // Write the data to prepend
    tmp.write_all(&data)?;
    // Copy the rest of the source file
    io::copy(&mut src, &mut tmp)?;
    fs::remove_file(&file_path)?;
    fs::rename(&tmp_path, &file_path)?;
    Ok(())
}
