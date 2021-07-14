CREATE TABLE users(
	member_id INT primary key,
	member_group_id INT,
	name text NOT NULL
);

CREATE TABLE games(
	id INT UNIQUE,
	domain_name TEXT PRIMARY KEY,
	name TEXT,
	approved_date DATETIME,
	authors INT,
	downloads BIGINT,
	file_count BIGINT,
	file_endorsements INT,
	file_views BIGINT,
	forum_url text,
	genre text,
	mods INT,
	nexusmods_url TEXT
);

CREATE TABLE categories(
	category_id INT,
	domain_name text NOT NULL,
	name TEXT,

	FOREIGN KEY (domain_name) REFERENCES games(id)
);

CREATE UNIQUE INDEX idx_categories_game_id ON categories(domain_name, category_id);

CREATE TABLE mods(
	id TEXT primary key,
	domain_name text NOT NULL,
	mod_id INT,
	etag text not null,

	uid INT,
	game_id INT,

	name text,
	version text,
	category_id INT,
	summary text,
	description text,
	picture_url text,
	available BOOLEAN default TRUE,
	status text,
	allow_rating BOOLEAN,
	contains_adult_content BOOLEAN default FALSE,

	author text NOT NULL,
	uploaded_by text NOT NULL,
	uploaded_users_profile_url text,
	user_id INT,
	endorsement_count INT,
	nexus_created timestamp,
	nexus_updated timestamp,

	created DATETIME NOT NULL default (datetime('now', 'utc')),
	modified DATETIME NOT NULL default (datetime('now', 'utc')),
	deleted DATETIME,

	FOREIGN KEY (user_id) REFERENCES users(member_id),
	FOREIGN KEY (domain_name) REFERENCES games(id)
);

CREATE UNIQUE INDEX idx_mods_game_mod ON mods(domain_name, mod_id);

CREATE TABLE tracked(
    mod_id INT,
    domain_name text NOT NULL,

    FOREIGN KEY (domain_name) REFERENCES games(id)
);

CREATE TABLE endorsements(
    domain_name text NOT NULL,
    mod_id INT,
    status text NOT NULL,
    version text,
    date INT,

	FOREIGN KEY (domain_name) REFERENCES games(id)
);
