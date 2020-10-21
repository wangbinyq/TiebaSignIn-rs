#[macro_use]
extern crate log;

use anyhow::{bail, Result};
use reqwest::{blocking::Client, header};
use serde::Deserialize;

const LIKE_URL: &str = "https://tieba.baidu.com/mo/q/newmoindex";
const TBS_URL: &str = "http://tieba.baidu.com/dc/common/tbs";
const SIGN_URL: &str = "http://c.tieba.baidu.com/c/c/forum/sign";

struct App {
    idx: usize,
    client: Client,
    tbs: String,
}

#[derive(Deserialize)]
struct TbsRes {
    is_login: i32,
    tbs: String,
}

#[derive(Deserialize)]
struct FollowResLikeForum {
    like_forum: Vec<FollowResLikeForumName>,
}

#[derive(Deserialize)]
struct FollowResLikeForumName {
    forum_name: String,
}

#[derive(Deserialize)]
struct FollowRes {
    data: FollowResLikeForum,
}

#[derive(Deserialize)]
struct SignRes {
    error_code: String,
    error_msg: String,
}

impl App {
    pub fn new(bduss: &str, idx: usize) -> Self {
        let mut headers = header::HeaderMap::new();

        let user_agent = header::HeaderValue::from_static("Mozilla/5.0 (Windows NT 6.1; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/39.0.2171.71 Safari/537.36");
        let cookie = header::HeaderValue::from_str(&format!("BDUSS={}", bduss)).unwrap();
        headers.insert(header::USER_AGENT, user_agent);
        headers.insert(header::COOKIE, cookie);

        let client = Client::builder().default_headers(headers).build().unwrap();

        Self {
            idx,
            client,
            tbs: "".into(),
        }
    }

    pub fn run(&mut self) {
        match self.get_tbs().and_then(|_| self.get_follows()) {
            Ok(follows) => {
                info!("开始签到...");
                let total = follows.len();
                let success = follows
                    .iter()
                    .fold(0, |acc, follow| match self.run_sign(follow) {
                        Ok(_) => {
                            info!("{} 签到成功", follow);
                            acc + 1
                        }
                        Err(err) => {
                            error!("{} 签到失败: {}", follow, err);
                            acc
                        }
                    });
                info!(
                    "第 {} 个账号签到完成, 成功 {} 个, 失败: {} 个",
                    self.idx,
                    success,
                    total - success
                );
            }
            Err(err) => error!("第 {} 个账号签到失败: {}", self.idx, err),
        }
    }

    fn get_tbs(&mut self) -> Result<()> {
        info!("第 {} 个账号登陆中...", self.idx);

        let response: TbsRes = self.client.get(TBS_URL).send()?.json()?;

        if response.is_login != 1 {
            bail!("登录失败")
        } else {
            self.tbs = response.tbs;
            info!("登录成功");
            Ok(())
        }
    }

    fn get_follows(&self) -> Result<Vec<String>> {
        info!("开始获取贴吧列表...");
        let response: FollowRes = self.client.get(LIKE_URL).send()?.json()?;
        let follows: Vec<String> = response
            .data
            .like_forum
            .into_iter()
            .map(|f| f.forum_name)
            .collect();

        info!("贴吧列表获取成功, 共 {} 个!!!", follows.len());

        Ok(follows)
    }

    fn run_sign(&self, follow: &str) -> Result<()> {
        let sign = format!("kw={}tbs={}tiebaclient!!!", follow, self.tbs);
        let sign: md5::Digest = md5::compute(sign);
        let body = format!("kw={}&tbs={}&sign={:x}", follow, self.tbs, sign);

        let res = self
            .client
            .post(SIGN_URL)
            .body(body)
            .send()?
            .json::<SignRes>()?;

        if res.error_code == "0" {
            Ok(())
        } else {
            bail!(res.error_msg)
        }
    }
}

fn main() -> Result<()> {
    env_logger::init();

    let bduss_list = option_env!("BDUSS").expect("请设置BDUSS");

    bduss_list
        .split("&")
        .enumerate()
        .for_each(|(idx, bduss)| App::new(bduss, idx + 1).run());

    Ok(())
}
