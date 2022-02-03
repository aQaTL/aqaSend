FROM ubuntu:18.04

RUN apt update && \
	apt install -y \
		curl \
		git \
		lsb-release \
		wget \
		software-properties-common && \
	wget https://apt.llvm.org/llvm.sh && \
	chmod +x llvm.sh && \
	./llvm.sh 13 all && \
	rm llvm.sh && \
	curl \
		--proto \
		'=https' \
		--tlsv1.2 \
		-sSf \
		-o \
		/rustup.sh \
		https://sh.rustup.rs && \
	chmod +x /rustup.sh && \
	/rustup.sh \
		-y \
		--default-host \
		x86_64-unknown-linux-gnu \
		--default-toolchain \
		stable \
		--profile \
		default && \
	rm /rustup.sh && \
	curl \
		-L \
		-o \
		node.tar.xz \
		https://nodejs.org/dist/v16.13.2/node-v16.13.2-linux-x64.tar.xz && \
	tar -xvf node.tar.xz && \
	rm node.tar.xz

CMD ["/bin/bash", "/aqaSend/docker/aqasend.sh"]
