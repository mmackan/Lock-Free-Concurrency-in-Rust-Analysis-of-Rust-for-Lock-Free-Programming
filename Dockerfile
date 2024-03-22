FROM ubuntu:mantic-20240216

RUN apt-get update

# Basic utilities
RUN apt-get install -y curl git

## Taken from the c++ project

# Build essentials
RUN apt-get install -y clang-17
RUN apt-get install -y libc++abi1-17 libc++abi-17-dev libc++-17-dev
RUN apt-get install -y cmake pkg-config autoconf

# jemalloc
RUN curl -L -o jemalloc.tar.gz https://github.com/jemalloc/jemalloc/archive/refs/tags/5.3.0.tar.gz
RUN tar -xzf jemalloc.tar.gz && rm jemalloc.tar.gz
WORKDIR /jemalloc-5.3.0
RUN ./autogen.sh && ./configure
RUN make -j4
RUN make install
WORKDIR /
RUN rm -rf /jemalloc-5.3.0

# Python
RUN apt-get install -y python3 python3-all
RUN apt-get install -y python3-pip
RUN pip3 install --break-system-packages numpy
RUN pip3 install --break-system-packages pandas
RUN pip3 install --break-system-packages matplotlib

ENV CC="clang-17"
ENV CXX="clang++-17"

## End c++ segment

## Rustup 
RUN curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

RUN rustup toolchain install nightly-2024-02-01
RUN cargo install cargo-make
RUN cargo install hyperfine

RUN apt-get clean

# Hack to get the right clang version set in the interactive environment
#RUN echo "export CC='clang-17'\n" >> /root/.bashrc
#RUN echo "export CXX='clang++-17'\n" >> /root/.bashrc

# Copy the repo into the image
COPY --link . /work
WORKDIR /work/rust-queues

ENTRYPOINT [ "/bin/bash"]