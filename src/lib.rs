use rand::random;
use serde::Deserialize;
use worker::*;

const HTML: &str = include_str!("./index.html");

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let router = Router::new();
    router
        .get_async(
            "/",
            |_, _| async move { Response::ok("Still Running!") },
        )
        .post_async("/upload", |mut req, _: RouteContext<()>| {
            let env = env.clone();
            async move {
                let secret = if let Ok(var) = env.var("SECRET") {
                    var.to_string()
                } else {
                    return Response::error("SECRET not found", 500);
                };
                let authorization = if let Ok(headers) = req.headers().get("Authorization") {
                    if let Some(headers) = headers {
                        headers
                    } else {
                        return Response::error("Authorization not found", 500);
                    }
                } else {
                    return Response::error("Authorization not found", 500);
                };
                if authorization == secret {
                    let body = if let Ok(text) = req.text().await {
                        text
                    } else {
                        return Response::error("Body not found", 500);
                    };

                    let db_result = if let Ok(db_result) =
                        write_db(&env.d1("rsbench").unwrap(), body).await
                    {
                        db_result
                    } else {
                        return Response::error("DB not found", 500);
                    };

                    Response::ok(db_result.to_string())
                } else {
                    Response::error("Unauthorized", 401)
                }
            }
        })
        .get_async("/:id/raw", |_req, ctx| {
            let env = env.clone();
            async move {
                {
                    let id = if let Some(id) = ctx.param("id") {
                        id
                    } else {
                        return Response::error("ID not found", 500);
                    };

                    let db_result = if let Ok(db_result) = read_db(&env.d1("rsbench").unwrap(), String::from(id)).await {
                        db_result
                    } else {
                        return Response::error("DB not found", 500);
                    };

                    Response::ok(db_result)
                }
            }
        })
        .get_async("/:id", |_req, ctx| {
            let env = env.clone();
            async move {
                let id = if let Some(id) = ctx.param("id") {
                id
            } else {
                return Response::error("ID not found", 500);
            };
                let db_result = if let Ok(db_result) = read_db(&env.d1("rsbench").unwrap(), id.to_string()).await {
                    db_result
                } else {
                    return Response::error("DB not found", 500);
                };

                let html_replaced = &*HTML.replace("RSBENCH_TEST_RESULT", &db_result);
                let html_replaced = html_replaced.replace("RAW_URL_HERE", format!("/{}/raw", id).as_str());

                Response::from_html(html_replaced)
            }
        })
        .run(req, env.clone())
        .await
}

async fn write_db(database: &D1Database, _string: String) -> Result<String> {
    use uuid::Uuid;
    let data: [u8; 16] = random();
    let random_uuid = Uuid::new_v8(data).to_string();

    let command = format!(
        "INSERT INTO main (id, content) VALUES ('{}', '{}');",
        random_uuid,
        _string
    );
    let command_d1 = database.prepare(&command);
    let res = command_d1.run().await?;

    return if res.success() {
        Ok(random_uuid)
    } else {
        Err(Error::from("Can not insert data"))
    }
}

async fn read_db(database: &D1Database, id: String) -> Result<String> {
    let command = format!("SELECT content FROM main WHERE id = '{}';", id);
    let command_d1 = database.prepare(&command);
    let res = command_d1.run().await?;

    #[derive(Deserialize)]
    struct TestResult {
        content: String,
    }

    match res.results::<TestResult>() {
        Ok(res) => {
            let res = if let Some(first) = res.first() {
                first.content.clone()
            } else {
                return Err(Error::from("Can not get content"));
            };
            Ok(res)
        }
        Err(e) => Err(e),
    }
}