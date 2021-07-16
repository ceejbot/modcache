CREATE TABLE users(
	member_id INT primary key,
	member_group_id INT,
	name text NOT NULL
);

CREATE TABLE authn_user(
	user_id INT PRIMARY KEY,
	email TEXT,
	is_premium TINYINT,
	is_supporter TINYINT,
	name TEXT,
	profile_url TEXT
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
	domain_name text NOT NULL,
	mod_id INT NOT NULL,
	etag text, /* not part of nexus data set */

	uid INT NOT NULL,
	game_id INT NOT NULL,

	name TEXT,
	version TEXT,
	category_id INT NOT NULL,
	summary TEXT,
	description TEXT,
	picture_url TEXT,
	status TEXT,
	available INT DEFAULT 1,
	allow_rating INT DEFAULT 0,
	contains_adult_content INT DEFAULT 0,

	author TEXT NOT NULL,
	uploaded_by TEXT NOT NULL,
	uploaded_users_profile_url TEXT,
	user_id INT,
	endorsement_count INT DEFAULT 0,
	nexus_created timestamp,
	nexus_updated timestamp,

	/* FOREIGN KEY (user_id) REFERENCES users(member_id), */
	FOREIGN KEY (domain_name) REFERENCES games(id)
);

CREATE UNIQUE INDEX idx_mods_game_mod ON mods(domain_name, mod_id);

CREATE TABLE tracked(
    domain_name text NOT NULL,
    mod_id INT
);
CREATE UNIQUE INDEX idx_tracked_game_mod ON tracked(domain_name, mod_id);

CREATE TABLE endorsements(
    domain_name text NOT NULL,
    mod_id INT,
    status text NOT NULL,
    version text,
    date INT,

	FOREIGN KEY (domain_name) REFERENCES games(id)
);
