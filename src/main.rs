extern crate file;
extern crate oauth_client as oauth;
extern crate yandex_translate as yandex;
extern crate json;
extern crate regex;

use oauth::Token;
use json::JsonValue;
use regex::Regex;
use yandex::client::YandexTranslate;
use yandex::answer::GetInfo;

struct Rumblr {
    tumblr_key: String,
    tumblr_secret: String,
    tumblr_token: String,
    tumblr_token_secret: String,
}
impl Rumblr {
    fn from_secret_folder(folder: &str) -> Rumblr {
        // read secret data
        let (tumblr_key, tumblr_secret) = {
            let text = file::get_text(format!("{}/tumblr_key.txt", folder))
                .expect("failed to read secret/tumblr_key.txt");
            let mut lines = text.split("\n");
            (
                String::from(lines.next().expect("tumblr_key line 1 missing")),
                String::from(lines.next().expect("tumblr_key line 2 missing"))
            )
        };


        let (tumblr_token, tumblr_token_secret) = {
            let text = file::get_text(format!("{}/tumblr_token.txt", folder))
                .expect("failed to read secret/tumblr_token.txt");
            let mut lines = text.split("\n");
            (
                String::from(lines.next().expect("tumblr_token line 1 missing")),
                String::from(lines.next().expect("tumblr_token line 2 missing"))
            )
        };

        // construct
        Rumblr {
            tumblr_key,
            tumblr_secret,
            tumblr_token,
            tumblr_token_secret
        }
    }

    fn oauth_token(&self) -> Token {
        Token::new(&self.tumblr_token[..], &self.tumblr_token_secret[..])
    }

    fn get_total_posts(&self, blog: &str) -> i64 {
        let b = oauth::get(
            &format!(
                "https://api.tumblr.com/v2/blog/{}/posts/photo?api_key={}&blog-identifier={}&api_key={}",
                blog, blog, self.tumblr_secret, self.tumblr_key
            ),
            &self.oauth_token(), None, None
        ).expect("http get fail");
        let s = String::from_utf8(b).expect("bytes to string fail");
        let j = json::parse(&s).expect("string to json fail");
        match j {
            JsonValue::Object(obj) => match obj["response"] {
                JsonValue::Object(ref response) => {
                    response["total_posts"].as_i64().expect("get total_posts as int fail")
                },
                _ => panic!("get response fail")
            },
            _ => panic!("json to object fail")
        }
    }

    fn get_posts(&self, blog: &str, offset: i64, number: i64) -> Vec<Post> {
        let b = oauth::get(
            &format!(
                "https://api.tumblr.com/v2/blog/{}/posts/photo?api_key={}&blog-identifier={}\
                &api_key={}&offset={}&limit={}&format=markdown",
                blog, self.tumblr_secret, blog, self.tumblr_key, offset, number
            ),
            &self.oauth_token(), None, None
        ).expect("http get fail");
        let s = String::from_utf8(b).expect("bytes to string fail");
        let j = json::parse(&s).expect("string to json fail");
        match j {
            JsonValue::Object(obj) => match obj["response"] {
                JsonValue::Object(ref response) => {
                    match response["posts"] {
                        JsonValue::Array(ref posts) => {
                            let mut vec = Vec::new();
                            for post in posts.iter() {
                                if let &JsonValue::Object(ref obj) = post {
                                    match obj["type"].as_str().unwrap() {
                                        "photo" => {
                                            let blog_name = String::from(obj["blog_name"].as_str()
                                                .expect("blog_name as str fail"));

                                            let caption = String::from(obj["caption"].as_str()
                                                .expect("caption as str fail"));
                                            let strip = Regex::new("<[^>]*>").unwrap();
                                            let caption = String::from(&*strip.replace_all(&caption[..], ""));

                                            let post_url = String::from(obj["post_url"].as_str()
                                                .expect("post_url as str fail"));
                                            let reblog_key = String::from(obj["reblog_key"].as_str()
                                                .expect("reblog_key as str fail"));
                                            let id = obj["id"].as_u64().expect("id as u64 fail");
                                            let tags =
                                                if let JsonValue::Array(ref tags) = obj["tags"] {
                                                    let mut vec = Vec::new();
                                                    for tag in tags.iter() {
                                                        vec.push(String::from(tag.as_str()
                                                            .expect("tag as str fail")))
                                                    }
                                                    vec
                                                } else {
                                                    panic!("tags as array fail")
                                                };
                                            vec.push(Post::Photo {
                                                blog_name,
                                                caption,
                                                id,
                                                post_url,
                                                reblog_key,
                                                tags
                                            });
                                        },
                                        s => vec.push(Post::Other(String::from(s))),
                                    }
                                } else {
                                    panic!("post as object fail")
                                }
                            }
                            vec
                        },
                        _ => panic!("get posts fail")
                    }
                },
                _ => panic!("get response fail")
            },
            _ => panic!("json to object fail")
        }
    }

    fn reblog_post(&self, post: &Post, blog: &str, caption: &str) {
        oauth::post(
            &format!(
                "https://api.tumblr.com/v2/blog/{}/post/reblog?api_key={}&type={}&id={}&reblog_key={}&comment={}",
                blog, self.api_key, post.post_type(), post.id(), post.reblog_key(), caption
            ),
            &self.oauth_token(), None, None
        ).expect("reblog fail");
    }
}

#[derive(Clone, Debug)]
enum Post {
    Photo {
        blog_name: String,
        caption: String,
        id: u64,
        post_url: String,
        reblog_key: String,
        tags: Vec<String>
    },
    Other(String)
}
impl Post {
    fn post_type(&self) -> &'static str {
        match self {
            &Post::Photo { .. } => "photo",
            &Post::Other(..) => panic!("post type of other")
        }
    }

    fn id(&self) -> u64 {
        match self {
            &Post::Photo {
                id,
                ..
            } => id,
            &Post::Other(..) => panic!("post if of other")
        }
    }

    fn reblog_key(&self) -> String {
        match self {
            &Post::Photo {
                ref reblog_key,
                ..
            } => reblog_key.clone(),
            &Post::Other(..) => panic!("reblog_key if of other")
        }
    }

    fn caption(&self) -> String {
        match self {
            &Post::Photo {
                ref caption,
                ..
            } => caption.clone(),
            &Post::Other(..) => panic!("caption if of other")
        }
    }
}

const POSTS_PER_RUN: i64 = 3;
const START_OFFSET: i64 = 37000;

fn main() {
    // create rust tumblr connection
    let rumblr = Rumblr::from_secret_folder("secret");

    // get total number of posts
    let bot_posts_num = rumblr.get_total_posts("markv5engbot");
    let original_posts_num = rumblr.get_total_posts("markv5");
    println!("total markv5 posts: {}", original_posts_num);

    // get the right chunk of russian posts
    let posts = rumblr.get_posts(
        "markv5",
        original_posts_num - bot_posts_num - POSTS_PER_RUN - START_OFFSET,
        POSTS_PER_RUN
    );

    // create the yandex translator
    let yandex_key = file::get_text("secret/yandex_key.txt")
        .expect("failed to read secret/yandex_key.txt");
    let translate = YandexTranslate::new()
        .set_apikey(yandex_key);

    // for each post, backwards
    for post in posts.into_iter().rev()
        .filter(|p| p.post_type() == "photo"){

        println!("{}", post.caption());

        // translate
        let english = translate.translate(vec!(&post.caption()[..]), "ru-en");
        let english = match english {
            yandex::answer::Answer::Translate(trans) => trans.get_message(),
            _ => panic!("yandex translate fail")
        };
        println!("->{}", english);

        // reblog
        rumblr.reblog_post(
            &post,
            "markv5engbot",
            &format!(
                "{}\n(I am a bot)",
                english
            )[..]
        )

    }

}
