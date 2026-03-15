# FastC Playground - CapRover Deployment

Deploy the FastC Playground as a web service using CapRover.

## Quick Deploy to CapRover

1. **Create an app in CapRover dashboard**
   - App name: `fastc-playground` (or your choice)
   - Enable HTTPS

2. **Deploy via CLI**
   ```bash
   # From the project root
   caprover deploy -a fastc-playground
   ```

   Or use the tarball method:
   ```bash
   tar -cvf deploy.tar --exclude='target' --exclude='node_modules' --exclude='.git' .
   caprover deploy -a fastc-playground -t ./deploy.tar
   ```

3. **Access your playground**
   - URL: `https://fastc-playground.your-captain-domain.com`

## Local Docker Testing

Build and run locally before deploying:

```bash
# From project root
docker build -f demo/Dockerfile -t fastc-playground .

# Run the container
docker run -p 3000:3000 fastc-playground

# Open http://localhost:3000
```

## Using Docker Compose

```bash
cd demo
docker-compose up --build
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `FASTC_RUNTIME` | `/app/runtime` | Path to FastC runtime headers |
| `PORT` | `3000` | Server port (use `--port` flag) |

## Resource Requirements

- **Build**: ~2GB RAM, ~5GB disk (Rust compilation)
- **Runtime**: ~128MB RAM, ~100MB disk

## Security Notes

The playground compiles and executes user-provided C code. The current setup:
- Runs in an isolated container
- Has a 5-second execution timeout
- No network access from executed code
- Temporary files cleaned after each run

For production, consider:
- Running with `--read-only` filesystem
- Using `seccomp` profiles
- Setting memory/CPU limits
- Using a dedicated execution sandbox (e.g., gVisor, Firecracker)
