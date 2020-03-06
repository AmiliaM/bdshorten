use actix_web::{App, HttpServer, Responder, web};
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

struct Link {
	id:          i32,
	slug:        String,
	destination: Url,
	created:     DateTime<Utc>,
	expiry:      DateTime<Utc>,
	deleted:     bool,
	token:       i32,
}

#[derive(Serialize, Deserialize)]
struct APILink {
	slug:        Option<String>,
	destination: Url,
	created:     DateTime<Utc>,
	expiry:      Option<DateTime<Utc>>,
}

struct Token {
	id:      i32,
	token:   String,
	auth:    i16,
	user:    String,
	created: String,
}

async fn handle_index() -> impl Responder {
	web::Json(APILink {
		slug: Some("1234".to_string()),
		destination: Url::parse("http://google.com").unwrap(),
		created: Utc.timestamp(1431648000, 0),
		expiry: None
	})
}

async fn handle_redirect() -> impl Responder {
	// get link by slug from database
	// return 301 with destination
	"Redirct"
}

// /links/
async fn handle_get_links() -> impl Responder {
	// get all links owned by that user (see docs)
	// return in json list (200)
	""
}

async fn handle_new_link() -> impl Responder {
	// convert received object from apilink to dblink
	// enter into database
	// return link and 201
	""
}

// /links/{link}
async fn handle_get_link() -> impl Responder {
	// get link by slug from database
	// convert from dblink to apilink
	// return link and 200
	""
}

async fn handle_delete_link() -> impl Responder {
	// get link by slug from database
	// mark link as deleted
	// return 204
	""
}

//tokens
async fn handle_new_token() -> impl Responder {
	// return 501
	""
}

async fn handle_new_invite() -> impl Responder {
	// return 501
	""
}


#[actix_rt::main]
async fn main() -> std::io::Result<()> {
	HttpServer::new(|| { App::new()
		.route("/",             web::get().to(handle_index))
		.route("/{link}",       web::get().to(handle_redirect))
		.route("/links",        web::get().to(handle_get_links))
		.route("/links",        web::post().to(handle_new_link))
		.route("/links/{link}", web::get().to(handle_get_link))
		.route("/links/{link}", web::delete().to(handle_delete_link))
		.route("/invite/{id}",  web::get().to(handle_new_token))
		.route("/invites",      web::post().to(handle_new_invite))
	})
	.bind("127.0.0.1:8088")?
	.run()
	.await
}
