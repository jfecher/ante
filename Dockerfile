FROM ubuntu

RUN apt-get update && \
    apt-get -y install software-properties-common && \
    add-apt-repository -y ppa:ubuntu-toolchain-r/test && \
    apt-get update && \
    apt-get -y install g++-7 gcc-7 cmake wget git make bison && \
    export CC='gcc-7' && \ 
    export CXX='g++-7' && \
    update-alternatives --install /usr/bin/gcc gcc /usr/bin/gcc-7 60 && \
    update-alternatives --install /usr/bin/g++ g++ /usr/bin/g++-7 60 && \
    mkdir /home/ante && \
    git clone https://github.com/jfecher/ante.git /home/ante && \
    cd /home/ante && \
    cmake . && \
    make -j 4
