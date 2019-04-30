CREATE TABLE public.links (
    id SERIAL PRIMARY KEY,
    symbol text UNIQUE NOT NULL,
    destination text NOT NULL,
    "timestamp" timestamp with time zone DEFAULT current_timestamp NOT NULL,
    expiry timestamp with time zone,
    deleted boolean DEFAULT false NOT NULL,
    CONSTRAINT links_check CHECK ((expiry > "timestamp"))
);