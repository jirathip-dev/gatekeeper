echo "$*" >> calls
n=$(wc -l < calls | tr -d ' ')
mkdir -p target/nextest/ci
cp "run$n.xml" target/nextest/ci/junit.xml
if grep -q "<failure" "run$n.xml"; then exit 100; fi
