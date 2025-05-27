set PGPASSWORD=MetricsOne
psql -h 127.0.0.1 -p 5432 -U api -f init.sql metrics-one
