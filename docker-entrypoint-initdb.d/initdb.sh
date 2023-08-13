set -e
psql -U admin sampledb <<EOSQL
CREATE TABLE users (
  id         SERIAL PRIMARY KEY,
  name	     VARCHAR(255) NOT NULL,
  email      VARCHAR(255) NOT NULL,
  created_at timestamp NOT NULL default current_timestamp,
  updated_at timestamp NOT NULL default current_timestamp
);
EOSQL
