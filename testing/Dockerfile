FROM ros:humble

RUN apt-get update && apt-get install -y \
  vim \
  git \
  curl \
  build-essential \
  clang

RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
RUN echo 'source $HOME/.cargo/env' >> /root/.bashrc

RUN echo "source /opt/ros/humble/setup.bash" >> /root/.bashrc

COPY hx /usr/bin/hx

# RUN /root/.cargo/bin/cargo install --locked bacon

ENTRYPOINT ["/bin/bash"]
