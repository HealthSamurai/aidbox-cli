# rm -rf target
docker build -t aidbox/aidbox-cli:0.0.1-RC1 .
docker push aidbox/aidbox-cli:0.0.1-RC1
