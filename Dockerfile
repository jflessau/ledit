FROM clux/muslrust:stable as build
RUN apt-get -yq update && apt-get -yqq install openssh-client

COPY . .

RUN eval `ssh-agent -s` && \
  # ssh-add /root/.ssh/id_rsa && \
  cargo build --target x86_64-unknown-linux-musl --release

# copy important stuff to smaller base image
FROM alpine
COPY --from=build /volume/target/x86_64-unknown-linux-musl/release/ledit /
COPY ./txt /txt

CMD ["/ledit"]