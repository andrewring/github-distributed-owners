FROM scratch
ARG TARGETARCH
COPY github-distributed-owners-linux-${TARGETARCH} /github-distributed-owners
ENTRYPOINT ["/github-distributed-owners"]
