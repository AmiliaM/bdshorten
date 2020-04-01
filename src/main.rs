//use actix_http::httpmessage::HttpMessage;
use actix_web::{App, HttpRequest, HttpResponse, HttpServer, Responder, http::{self, StatusCode, header}, web::{self, Json, Path}};
use base64;
use chrono::{DateTime, TimeZone, Utc};
use failure::Error;
use rand::{thread_rng, RngCore};
use serde::{Deserialize, Serialize};
use tokio_postgres::{self as postgres, NoTls};
//use postgres_types::FromSql;
use url::Url;

use std::collections::HashMap;

/*#[derive(FromSql)]
struct Link {
	id:          i32,
	slug:        String,
	destination: String,
	created:     DateTime<Utc>,
	expiry:      Option<DateTime<Utc>>,
	deleted:     bool,
	token:       i32,
}*/

#[derive(Serialize, Deserialize)]
struct APILink {
	slug:        Option<String>,
	destination: Url,
	#[serde(skip_deserializing)]
	created:     Option<DateTime<Utc>>,
	expiry:      Option<DateTime<Utc>>,
}

/*struct Token {
	id:      i32,
	token:   String,
	auth:    i16,
	desc:    String,
	created: DateTime<Utc>,
}*/

type Db = web::Data<postgres::Client>;

fn new_ident(len: usize) -> String {
	let mut bytes = Vec::new();
	bytes.resize(len, 0);

	thread_rng().fill_bytes(&mut bytes);
	let mut x = base64::encode_config(&bytes, base64::URL_SAFE_NO_PAD);

	x.truncate(len);
	x
}

// /
async fn handle_index() -> impl Responder {
	//FIXME: meaningful return
	Json(APILink {
		slug: Some("1234".to_string()),
		destination: Url::parse("http://google.com").unwrap(),
		created: Some(Utc.timestamp(1431648000, 0)),
		expiry: None
	})
}

// /go/{link}
async fn handle_redirect(db: Db, ident: Path<String>) -> impl Responder {
	let dest = Url::parse(
		db.get_ref().query_one("SELECT destination FROM validlinks WHERE slug = $1", &[&*ident]).await
		.unwrap().get(0)).unwrap();

	HttpResponse::MovedPermanently()
		.header(http::header::LOCATION, dest.as_str())
		.finish()
}

// /links/
async fn handle_get_links(db: Db, r: HttpRequest) -> impl Responder {
	let token = db.get_ref().query_one(
		"SELECT id, auth FROM tokens WHERE token = $1",
		&[&r.headers().get("Authorization").unwrap().to_str().unwrap()]).await.unwrap();
	let (id, auth): (i32, i16) = (token.get(0), token.get(1));

	let resp;
	if auth == 3 {
		resp = db.get_ref().query("SELECT slug, destination, created, expiry FROM links", &[]).await.unwrap();
	} else {
		resp = db.get_ref().query(
			"SELECT slug, destination, created, expiry FROM validlinks WHERE token=$1",
			&[&id]).await.unwrap();
	}

	let mut links: Vec<APILink> = Vec::with_capacity(resp.len());
	for link in resp {
		links.push(APILink {
			slug:        link.get(0),
			destination: Url::parse(link.get(1)).unwrap(),
			created:     link.get(2),
			expiry:      link.get(3),
		});
	}
	Json(links)
}

async fn handle_new_link(db: Db, body: Json<APILink>, r: HttpRequest) -> impl Responder {
	let APILink {
		slug,
		destination,
		expiry,
		..
	} = body.into_inner();

	let token = db.get_ref().query_one(
		"SELECT id, auth FROM tokens WHERE token = $1",
		&[&r.headers().get("Authorization").unwrap().to_str().unwrap()]).await.unwrap();
	let (id, auth): (i32, i16) = (token.get(0), token.get(1));

	let slug = match auth {
		2 => {
			let mut s = slug.unwrap_or_else(| | new_ident(6));
			if s.len() < 6 {
				s += &new_ident(6 - s.len());
			}
			s
		},
		3 => slug.unwrap_or_else(| | new_ident(4)),
		_ => new_ident(8),
	};

	db.get_ref().execute(
	  "INSERT INTO links (slug, destination, expiry, token) VALUES ($1, $2, $3, $4)",
	  &[&slug, &destination.as_str(), &expiry, &id]).await.unwrap();

	let created = db.get_ref()
	    .query_one("SELECT created FROM validlinks WHERE slug = $1", &[&slug]).await.unwrap().get(0);

	Json( APILink {
		slug: Some(slug),
		destination,
		created,
		expiry,
	})
}

// /links/{link}
async fn handle_get_link(db: Db, ident: Path<String>, r: HttpRequest) -> impl Responder {
	let token = db.get_ref().query_one(
		"SELECT id, auth FROM tokens WHERE token = $1",
		&[&r.headers().get("Authorization").unwrap().to_str().unwrap()]).await.unwrap();
	let (id, auth): (i32, i16) = (token.get(0), token.get(1));

	let link;
	if auth == 3 {
		link = db.get_ref().query_one(
			"SELECT slug, destination, created, expiry FROM validlinks WHERE slug = $1",
			&[&ident.as_str()]).await.unwrap();
	} else {
		link = db.get_ref().query_one(
			"SELECT slug, destination, created, expiry FROM validlinks WHERE slug = $1 AND token = $2",
			&[&ident.as_str(), &id]).await.unwrap();
	}

	Json(APILink {
		slug: link.get(0),
		destination: Url::parse(link.get(1)).unwrap(),
		created: link.get(2),
		expiry: link.get(3),
	})
}

async fn handle_delete_link(db: Db, ident: Path<String>, r: HttpRequest) -> impl Responder {
	let auth: i16 = db.get_ref().query_one(
		"SELECT auth FROM tokens WHERE token = $1",
		&[&r.headers().get("Authorization").unwrap().to_str().unwrap()]).await.unwrap().get(0);

	if auth == 3 {
		db.get_ref().execute(
			"UPDATE links SET deleted = True WHERE slug = $1",
			&[&ident.as_str()]).await.unwrap();
	} else {
		db.get_ref().execute(
			"UPDATE links SET deleted = True WHERE slug = $1 AND token = $2",
			&[&ident.as_str()]).await.unwrap();
	}

	HttpResponse::NoContent()
}

// /invite/{ident}
async fn handle_new_token(db: Db, token: Path<String>, body: Json<HashMap<String, String>>) -> impl Responder {
	let user = body.get("user").unwrap();

	let auth: i16 = db.get_ref().query_one(
		"SELECT auth FROM invites WHERE token = $1",
		&[&token.as_str()]).await.unwrap().get(0);

	let token = new_ident(42);

	let invites: Option<i32> = match auth {
		1 => Some(0),
		2 => Some(2),
		_ => None,
	};

	db.get_ref().execute(
		"INSERT INTO tokens (token, auth, descr, invites) VALUES ($1, $2, $3, $4)",
		&[&token, &auth, user, &invites]).await.unwrap();

	db.get_ref().execute(
		"UPDATE invites SET used = true WHERE token = $1",
		&[&token.as_str()]).await.unwrap();

	HttpResponse::Created().json(("token", token))
}

// /invites/
async fn handle_new_invite(db: Db, body: Json<HashMap<String, String>>, r: HttpRequest) -> impl Responder {
	let token = db.get_ref().query_one(
		"SELECT id, auth, invites FROM tokens WHERE token = $1",
		&[&r.headers().get("Authorization").unwrap().to_str().unwrap()]).await.unwrap();
	let (id, auth, invites): (i32, i16, i32) = (token.get(0), token.get(1), token.get(2));

	match auth {
		2 => {
			if invites > 0 {
				let invite = new_ident(32);
				db.get_ref().execute(
					"INSERT INTO invites (parent, token) VALUES ($1, $2) ",
					&[&id, &invite]).await.unwrap();
				db.get_ref().execute(
					"UPDATE tokens SET invites = invites - 1 WHERE id = $1",
					&[&id]).await.unwrap();
				return HttpResponse::Created().json(("invite-uri", invite));
			} else { return HttpResponse::Forbidden().body("No remaining invites") }
		},
		3 => {
			let invite = new_ident(32);
			let new_auth: i16 = body.get("auth-level").unwrap().parse().unwrap();
			db.get_ref().execute(
				"INSERT INTO invites (parent, token, auth) VALUES ($1, $2, $3)",
				&[&id, &invite, &new_auth]).await.unwrap();
			return HttpResponse::Created().json(("invite-uri", invite)); // FIXME: make uri
		},
		_ => {
			return HttpResponse::Forbidden().finish();
		},
	}
}

#[actix_rt::main]
async fn main() -> Result<(), Error> {
	env_logger::init();
	let (client, connection) = postgres::connect("host=localhost user=amilia dbname=linkshortener", NoTls).await?;
	let client = web::Data::new(client);
	tokio::spawn(async move {
		if let Err(e) = connection.await {
			eprintln!("connection error: {}", e);
		}
	});

	let bind = "127.0.0.1:8080";
	println!("Starting server at: {}", &bind);

	Ok(HttpServer::new(move || {App::new()
		.app_data(client.clone())
		.route("/",             web::get().to(handle_index))
		.route("/go/{link}",    web::get().to(handle_redirect))
		.route("/links",        web::get().to(handle_get_links))
		.route("/links",        web::post().to(handle_new_link))
		.route("/links/{link}", web::get().to(handle_get_link))
		.route("/links/{link}", web::delete().to(handle_delete_link))
		.route("/invite/{id}",  web::get().to(handle_new_token))
		.route("/invites",      web::post().to(handle_new_invite))
	})
	.bind(&bind)?
	.run()
	.await?)
}
