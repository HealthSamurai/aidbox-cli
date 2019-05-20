# rm -rf target
docker build -t aidbox/aidbox-cli:0.0.1-alpha .
docker push aidbox/aidbox-cli:0.0.1-alpha
