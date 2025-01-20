use serde::Deserialize;
use worker::*;

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
        .get_async("/:id", |_req, ctx| {
            let env = env.clone();
            async move {
                {
                    let id = if let Some(id) = ctx.param("id") {
                        id
                    } else {
                        return Response::error("ID not found", 500);
                    };

                    let id = if let Ok(id) = id.parse::<u32>() {
                        id
                    } else {
                        return Response::error("ID not found", 500);
                    };

                    let db_result = if let Ok(db_result) = read_db(&env.d1("rsbench").unwrap(), id).await {
                        db_result
                    } else {
                        return Response::error("DB not found", 500);
                    };

                    Response::ok(db_result)
                }
            }
        })
        .run(req, env.clone())
        .await
}

async fn write_db(database: &D1Database, _string: String) -> Result<u32> {
    let command = format!(
        "INSERT INTO main (content) VALUES ('{}') RETURNING id;",
        _string
    );
    let command_d1 = database.prepare(&command);
    let res = command_d1.run().await?;

    #[derive(Deserialize)]
    struct ID {
        id: u32,
    }

    match res.results::<ID>() {
        Ok(res) => {
            let res = if let Some(first) = res.first() {
                first.id
            } else {
                return Err(Error::from("Can not get id"));
            };
            Ok(res)
        }
        Err(e) => Err(e),
    }
}

async fn read_db(database: &D1Database, id: u32) -> Result<String> {
    let command = format!("SELECT content FROM main WHERE id = {};", id);
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
