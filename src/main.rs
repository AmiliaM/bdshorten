use actix_web::{App, HttpRequest, HttpResponse, HttpServer, Responder, http::header, web::{self, Json, Path}};
use base64;
use chrono::{DateTime, TimeZone, Utc};
use failure::Error;
use rand::{thread_rng, RngCore};
use serde::{Deserialize, Serialize};
use tokio_postgres::{self as postgres, NoTls};
use url::Url;

use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
struct APILink {
	slug:        Option<String>,
	destination: Url,
	#[serde(skip_deserializing)]
	created:     Option<DateTime<Utc>>,
	expiry:      Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
struct Invite { authlevel: Option<i16> }

type Db = web::Data<postgres::Client>;

fn new_ident(len: usize) -> String {
	let mut bytes = Vec::new();
	bytes.resize(len, 0);

	thread_rng().fill_bytes(&mut bytes);
	let mut x = base64::encode_config(&bytes, base64::URL_SAFE_NO_PAD);

	x.truncate(len);
	x
}

fn rows_to_links(rows: Vec<tokio_postgres::row::Row>) -> Vec<APILink> {
	let mut links: Vec<APILink> = Vec::with_capacity(rows.len());
	for link in rows {
		links.push(APILink {
			slug:        link.get(0),
			destination: Url::parse(link.get(1)).unwrap(),
			created:     link.get(2),
			expiry:      link.get(3),
		});
	}
	links
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
	let q = db.get_ref().query_opt("SELECT destination FROM validlinks WHERE slug = $1", &[&*ident]).await;
	let dest = match q {
		Ok(Some(dest)) => Url::parse(dest.get(0)).unwrap(),
		Ok(None) => return HttpResponse::NotFound().finish(),
		Err(_) => return HttpResponse::InternalServerError().finish(),
	};

	HttpResponse::MovedPermanently()
		.header(header::LOCATION, dest.as_str())
		.finish()
}

// /links/
async fn handle_get_links(db: Db, r: HttpRequest) -> impl Responder {
	let token = match r.headers().get("Authorization") {
		Some(h) => h.to_str().unwrap(),
		None => return HttpResponse::Unauthorized().header("WWW-Authenticate", "Bearer").finish(),
	};

	let q = db.get_ref().query_opt("SELECT id, auth FROM tokens WHERE token = $1", &[&token]).await;
	let (id, auth): (i32, i16) = match q {
		Ok(Some(tok)) => (tok.get(0), tok.get(1)),
		Ok(None) => return HttpResponse::NotFound().finish(),
		Err(_) => return HttpResponse::InternalServerError().finish(),
	};

	(if auth == 3 {
		db.get_ref().query(
			"SELECT slug, destination, created, expiry FROM links",
			&[]).await
	} else {
		db.get_ref().query(
			"SELECT slug, destination, created, expiry FROM validlinks WHERE token=$1",
			&[&id]).await
	}).map_or_else(
		|_| HttpResponse::InternalServerError().finish(),
		|v| HttpResponse::Ok().json(rows_to_links(v)))
}

async fn handle_new_link(db: Db, body: Json<APILink>, r: HttpRequest) -> impl Responder {
	let APILink {
		slug,
		destination,
		expiry,
		..
	} = body.into_inner();

	let token = match r.headers().get("Authorization") {
		Some(h) => h.to_str().unwrap(),
		None => return HttpResponse::Unauthorized().header("WWW-Authenticate", "Bearer").finish(),
	};

	let q = db.get_ref().query_opt("SELECT id, auth FROM tokens WHERE token = $1", &[&token]).await;
	let (id, auth): (i32, i16) = match q {
		Ok(Some(tok)) => (tok.get(0), tok.get(1)),
		Ok(None) => return HttpResponse::NotFound().finish(),
		Err(_) => return HttpResponse::InternalServerError().finish(),
	};

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

	let q = db.get_ref().execute(
		"INSERT INTO links (slug, destination, expiry, token) VALUES ($1, $2, $3, $4)",
		&[&slug, &destination.as_str(), &expiry, &id]).await;
	if let Err(_) = q {
		return HttpResponse::InternalServerError().finish() //FIXME: handling for if selected slug already taken (409)
	}

	let q = db.get_ref()
		.query_opt("SELECT created FROM validlinks WHERE slug = $1", &[&slug]).await;
	let created = match q {
		Ok(Some(row)) => row.get(0),
		Ok(None) => return HttpResponse::NotFound().finish(),
		Err(_) => return HttpResponse::InternalServerError().finish(),
	};

	HttpResponse::Created().json(APILink {
		slug: Some(slug),
		destination,
		created,
		expiry,
	})
}

// /links/{link}
async fn handle_get_link(db: Db, ident: Path<String>, r: HttpRequest) -> impl Responder {
	let token = match r.headers().get("Authorization") {
		Some(h) => h.to_str().unwrap(),
		None => return HttpResponse::Unauthorized().header("WWW-Authenticate", "Bearer").finish(),
	};

	let q = db.get_ref().query_opt("SELECT id, auth FROM tokens WHERE token = $1", &[&token]).await;
	let (id, auth): (i32, i16) = match q {
		Ok(Some(tok)) => (tok.get(0), tok.get(1)),
		Ok(None) => return HttpResponse::NotFound().finish(),
		Err(_) => return HttpResponse::InternalServerError().finish(),
	};

	let link = match if auth == 3 {
		db.get_ref().query_opt(
			"SELECT slug, destination, created, expiry FROM links WHERE slug = $1",
			&[&ident.as_str()]).await
	} else {
		db.get_ref().query_opt(
			"SELECT slug, destination, created, expiry FROM validlinks WHERE slug = $1 AND token = $2",
			&[&ident.as_str(), &id]).await
	} {
		Ok(Some(row)) => row,
		Ok(None) => return HttpResponse::NotFound().finish(),
		Err(_) => return HttpResponse::InternalServerError().finish(),
	};

	HttpResponse::Ok().json(APILink {
		slug: link.get(0),
		destination: Url::parse(link.get(1)).unwrap(),
		created: link.get(2),
		expiry: link.get(3),
	})
}

async fn handle_delete_link(db: Db, ident: Path<String>, r: HttpRequest) -> impl Responder {
	let token = match r.headers().get("Authorization") {
		Some(h) => h.to_str().unwrap(),
		None => return HttpResponse::Unauthorized().header("WWW-Authenticate", "Bearer").finish(),
	};

	let q = db.get_ref().query_opt("SELECT auth FROM tokens WHERE token = $1", &[&token]).await;
	let auth: i16 = match q {
		Ok(Some(tok)) => (tok.get(0)),
		Ok(None) => return HttpResponse::NotFound().finish(),
		Err(_) => return HttpResponse::InternalServerError().finish(),
	};

	match if auth == 3 {
		db.get_ref().execute(
			"UPDATE links SET deleted = true WHERE slug = $1",
			&[&ident.as_str()]).await
	} else {
		db.get_ref().execute(
			"UPDATE links SET deleted = true WHERE slug = $1 AND token = $2",
			&[&ident.as_str()]).await
	} {
		Ok(_) => HttpResponse::NoContent().finish(),
		Err(_) => HttpResponse::InternalServerError().finish()
	}
}

// /invite/{token}
async fn handle_new_token(db: Db, token: Path<String>, body: Json<HashMap<String, String>>) -> impl Responder {
	let user = match body.get("user") {
		Some(u) => u,
		None => return HttpResponse::BadRequest().finish(),
	};

	let q = db.get_ref().query_opt(
		"SELECT auth FROM invites WHERE token = $1 AND used = false",
		&[&token.as_str()]).await;
	let auth: i16 = match q {
		Ok(Some(auth)) => auth.get(0),
		Ok(None) => return HttpResponse::NotFound().finish(),
		Err(_) => return HttpResponse::InternalServerError().finish(),
	};

	let token = new_ident(42);

	let invites: Option<i32> = match auth {
		1 => Some(0),
		2 => Some(2),
		_ => None,
	};

	let q = db.get_ref().execute(
		"INSERT INTO tokens (token, auth, descr, invites) VALUES ($1, $2, $3, $4)",
		&[&token, &auth, user, &invites]).await;
	if let Err(_) = q {
		return HttpResponse::InternalServerError().finish(); //FIXME: handing for if token ident already used?
	}

	let q = db.get_ref().execute(
		"UPDATE invites SET used = true WHERE token = $1",
		&[&token.as_str()]).await;
	if let Err(_) = q {
		return HttpResponse::InternalServerError().finish();
	}

	HttpResponse::Created().json(("token", token))
}

// /invites/
async fn handle_new_invite(db: Db, body: Json<Invite>, r: HttpRequest) -> impl Responder {
	let token = match r.headers().get("Authorization") {
		Some(h) => h.to_str().unwrap(),
		None => return HttpResponse::Unauthorized().header("WWW-Authenticate", "Bearer").finish(),
	};

	let q = db.get_ref().query_opt("SELECT id, auth, invites FROM tokens WHERE token = $1", &[&token]).await;
	let (id, auth, invites): (i32, i16, Option<i32>) = match q {
		Ok(Some(tok)) => (tok.get(0), tok.get(1), tok.get(2)),
		Ok(None) => return HttpResponse::NotFound().finish(),
		Err(_) => return HttpResponse::InternalServerError().finish(),
	};

	match auth { //FIXME: more random token collision issues?
		2 => {
			if invites.unwrap_or(0) > 0 {
				let invite = new_ident(32);

				let q = db.get_ref().execute(
					"INSERT INTO invites (parent, token) VALUES ($1, $2)",
					&[&id, &invite]).await;
				if let Err(_) = q {
					return HttpResponse::InternalServerError().finish();
				}

				let q = db.get_ref().execute(
					"UPDATE tokens SET invites = invites - 1 WHERE id = $1",
					&[&id]).await;
				if let Err(_) = q {
					return HttpResponse::InternalServerError().finish();
				}

				HttpResponse::Created().json(("invite-uri", invite))
			} else { HttpResponse::Forbidden().body("No remaining invites") }
		},
		3 => {
			let invite = new_ident(32);
			let q = db.get_ref().execute(
				"INSERT INTO invites (parent, token, auth) VALUES ($1, $2, $3)",
				&[&id, &invite, &body.authlevel]).await;
			if let Err(_) = q {
				return HttpResponse::InternalServerError().finish();
			}

			HttpResponse::Created().json(("invite-uri", invite)) // FIXME: make uri
		},
		_ => {
			HttpResponse::Forbidden().finish()
		},
	}
}

#[actix_rt::main]
async fn main() -> Result<(), Error> {
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
		.route("/invite/{id}",  web::post().to(handle_new_token))
		.route("/invites",      web::post().to(handle_new_invite))
	})
	.bind(&bind)?
	.run()
	.await?)
}
