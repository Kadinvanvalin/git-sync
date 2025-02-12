use std::cell::RefCell;
use rouille::Request;
use rouille::Response;
use std::io;
use serde::{Deserialize, Serialize};
#[macro_use]
extern crate rouille;
#[derive(Deserialize, Serialize, Debug)]
pub struct Project {
    pub ssh_url_to_repo: String,
    pub path_with_namespace: String,
}

fn main() {
    
    rouille::start_server("0.0.0.0:80", move |request| {
        rouille::log(request, io::stdout(), || {
            router!(request,
                (GET) (/projects) => {
                    // When viewing the home page, we return an HTML document described below.
                    let page = request.get_param("page").unwrap();
                     if page == "1" {
            return Response::json(
                &vec![
                    Project{
                        ssh_url_to_repo: "git@localhost:myfakeslug/widgetmaker.git".to_string(),
                        path_with_namespace: "".to_string()
                    }
                ]
            );
        }
                Response::json(&Vec::<Project>::new())
                },
        (GET) (/myfakeslug/widgetmaker) => {
                   Response::text(format!("welcome to project page for {} {}", "myfakeslug", "widgetmaker")
                    ) 
                },
        _ => rouille::Response::empty_404()
        )
    })
    });
}