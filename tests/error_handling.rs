mod test_client;

use tophat::server::{
    accept,
    glitch::{Context, Glitch},
};

use test_client::TestClient;

const RESP_400: &str = "HTTP/1.1 400 Bad Request\r\ncontent-length: 0\r\n\r\n";
#[allow(dead_code)] // because of testing with and without anyhow errors
const RESP_500: &str = "HTTP/1.1 500 Internal Server Error\r\ncontent-length: 0\r\n\r\n";

#[test]
fn test_request_manually_create_glitch() {
    smol::block_on(async {
        let testclient = TestClient::new(
            "GET /foo/bar HTTP/1.1\r\nHost: example.org\r\n\r\n",
            RESP_400,
        );

        accept(testclient.clone(), |_req, resp_wtr| async move {
            "one".parse::<usize>()
                .map_err(|_| Glitch::bad_request())?;
            let done = resp_wtr.send().await.unwrap();

            Ok(done)
        })
        .await
        .unwrap();

        testclient.assert();
    });
}

#[test]
fn test_request_glitch_with_context() {
    // one test to see that just `?` works, and another to see that manual Glitch creation still
    // works even with anyhow enabled.

    // automatic
    smol::block_on(async {
        let testclient = TestClient::new(
            "GET /foo/bar HTTP/1.1\r\nHost: example.org\r\n\r\n",
            RESP_500,
        );

        accept(testclient.clone(), |_req, resp_wtr| async move {
            "one".parse::<usize>()?;
            let done = resp_wtr.send().await.unwrap();

            Ok(done)
        })
        .await
        .unwrap();

        testclient.assert();
    });

    // context
    smol::block_on(async {
        let testclient = TestClient::new(
            "GET /foo/bar HTTP/1.1\r\nHost: example.org\r\n\r\n",
            "HTTP/1.1 500 Internal Server Error\r\ncontent-length: 12\r\n\r\ncustom error",
        );

        accept(testclient.clone(), |_req, resp_wtr| async move {
            "one".parse::<usize>()
                .context("custom error")?;
            let done = resp_wtr.send().await.unwrap();

            Ok(done)
        })
        .await
        .unwrap();

        testclient.assert();
    });

    // manual
    smol::block_on(async {
        let testclient = TestClient::new(
            "GET /foo/bar HTTP/1.1\r\nHost: example.org\r\n\r\n",
            RESP_400,
        );

        accept(testclient.clone(), |_req, resp_wtr| async move {
            "one".parse::<usize>()
                .map_err(|_| Glitch::bad_request())?;
            let done = resp_wtr.send().await.unwrap();

            Ok(done)
        })
        .await
        .unwrap();

        testclient.assert();
    });
}

