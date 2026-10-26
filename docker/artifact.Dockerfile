# Artifact image for the faker WASM component — it ships the compiled
# component only; it is not a runnable service.
#
# Built by .github/workflows/release.yml with `docker buildx` for
# linux/amd64,linux/arm64. The build context is dist/ (which contains
# faker.wasm); FROM scratch with no RUN means no QEMU emulation is needed.
#
#   docker buildx build --platform linux/amd64,linux/arm64 \
#     -f docker/artifact.Dockerfile -t ghcr.io/perfscale/faker:latest dist

FROM scratch

COPY faker.wasm /faker.wasm

LABEL org.opencontainers.image.source="https://github.com/Perfscale/faker" \
      org.opencontainers.image.description="Perfscale/faker — deterministic fake-data generator WASM component for the Perfscale load-testing engine" \
      org.opencontainers.image.licenses="MIT OR Apache-2.0"
