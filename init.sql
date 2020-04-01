CREATE TABLE tokens (
	id integer PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
	token text UNIQUE NOT NULL,
	auth smallint DEFAULT 1 NOT NULL,
	descr text NOT NULL,
	created timestamptz DEFAULT CURRENT_TIMESTAMP NOT NULL,
	invites integer DEFAULT 0,
	CONSTRAINT token_len CHECK ((char_length(token) = 42))
);

CREATE TABLE links (
	id integer PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
	slug text UNIQUE NOT NULL,
	destination text NOT NULL,
	created timestamptz DEFAULT CURRENT_TIMESTAMP NOT NULL,
	expiry timestamptz,
	deleted boolean DEFAULT false NOT NULL,
	author integer NOT NULL REFERENCES tokens,
	CONSTRAINT links_check CHECK ((expiry > created))
);

CREATE TABLE invites (
	id integer PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
	parent integer NOT NULL REFERENCES tokens,
	token text UNIQUE NOT NULL,
	auth smallint DEFAULT 1 NOT NULL,
	created timestamptz DEFAULT CURRENT_TIMESTAMP NOT NULL,
	used boolean DEFAULT false NOT NULL,
	CONSTRAINT ident_len CHECK ((char_length(ident) = 32))
)

CREATE VIEW validlinks AS
	SELECT id,
		slug,
		destination,
		created,
		expiry,
		token
	FROM links
	WHERE (((expiry IS NULL) OR (expiry > CURRENT_TIMESTAMP)) AND (NOT deleted));
