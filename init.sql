CREATE TABLE tokens (
	id integer PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
	token text UNIQUE NOT NULL,
	auth smallint DEFAULT 1 NOT NULL,
	user text NOT NULL,
	created timestamptz DEFAULT CURRENT_TIMESTAMP NOT NULL,
	CONSTRAINT token_len CHECK ((char_length(token) = 42))
);

CREATE TABLE links (
	id integer PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
	slug text UNIQUE NOT NULL,
	destination text NOT NULL,
	created timestamptz DEFAULT current_timestamp NOT NULL,
	expiry timestamptz,
	deleted boolean DEFAULT false NOT NULL,
	token integer NOT NULL REFERENCES tokens,
	CONSTRAINT links_check CHECK ((expiry > created))
);

CREATE VIEW validlinks AS
	SELECT id,
		symbol,
		destination,
		created,
		expiry,
		token
	FROM links
	WHERE (((expiry IS NULL) OR (expiry > CURRENT_TIMESTAMP)) AND (NOT deleted));
