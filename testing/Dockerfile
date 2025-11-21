# FROM docker.io/library/arm64v8/ros:humble
FROM ros:humble

RUN apt-get update && apt-get install -y \
  vim \
  git \
  curl

RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
RUN echo 'source $HOME/.cargo/env' >> /root/.bashrc

RUN echo "source /opt/ros/humble/setup.bash" >> /root/.bashrc

COPY /home/jos/Vaults/Work/orientation_server /orientation_server

ENTRYPOINT ["/bin/bash"]
