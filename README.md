# Simple Rust based API template with Postgres integration and in-memory caching

## How to run for development

1. Clone the repository
2. Run a simple test Postgres container with `docker run --name postgres_temp_db -e POSTGRES_PASSWORD=postgres -d -p 5432:5432 postgres`
3. Export the following environment variables using `export` or by creating a `.env` file:
    - `RUST_LOG=rust_api=TRACE`
    - `API_WORKERS_COUNT=4`
    - `POSTGRES_DB_URL=postgres://postgres:postgres@localhost:5432/postgres`
    - `CACHE_MAX_CAPACITY=10000`
    - `CACHE_TIME_TO_LIVE=300`
    - `POSTGRES_DB_MAX_POOL_SIZE=100`
4. Run the application with `cargo run` to start the server on all interfaces on port 8686
5. Access the API at `http://localhost:8686` or public IP of the server on port 8686 and its endpoints
6. After you are done, stop and remove the containers with `docker stop postgres_temp_db && docker rm postgres_temp_db`


## Deployment

For production deployment, the template provides docker CI pipeline and `docker-compose` configuration files for easy deployment. And use the docker compose file to deploy the application.

## Contributing

Contributions are welcome! If you'd like to contribute to Rust-API Template, please follow these steps:

1. Fork the repository
2. Create a new branch for your feature or bug fix
3. Make your changes and commit them
4. Push your changes to your fork
5. Submit a pull request to the `main` branch of the original repository

Please make sure to follow the existing code style and add tests for any new features or bug fixes.

## License

Rust-API Template is released under the [MIT License](https://github.com/Neko-Nik/Rust-API-Template/blob/main/LICENSE). You are free to use, modify, and distribute this template for any purpose.
