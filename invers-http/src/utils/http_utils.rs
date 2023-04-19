#[derive(Debug)]
pub struct Request<'a> {
    pub method: String,
    pub path: String,
    pub headers: Vec<(&'a str, &'a str)>,
}

pub fn extract_request_from_tcp_stream(
    request_string: &str,
) -> Result<Request, String> {
    let lines = request_string.split("\r\n").collect::<Vec<&str>>();
    let first_line = lines.first().unwrap();
    let first_line_parts = first_line.split(' ').collect::<Vec<&str>>();

    let method = first_line_parts.first().unwrap();
    let path = first_line_parts.get(1).unwrap();

    let mut headers = Vec::new();

    for line in lines.iter().skip(1) {
        // Ignoring the last line as it may be empty or contain the body
        if line.is_empty() {
            break;
        }

        let line_parts = line.split(": ").collect::<Vec<&str>>();
        let header_name = line_parts.first().unwrap_or(&"");
        let header_value = line_parts.get(1).unwrap_or(&"");

        if line_parts.len() == 2 {
            headers.push((*header_name, *header_value));
        } else {
            println!("Skipping header: {}:{}", header_name, header_value);
        }
    }
    let request = Request {
        method: method.to_string(),
        path: path.to_string(),
        headers,
    };

    Ok(request)
}
