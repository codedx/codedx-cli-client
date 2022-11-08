/* CLI Client build/packaging pipeline. This does a build and generates build artifacts for the
 * supported platforms
 */

pipeline {
	options {
		buildDiscarder logRotator(artifactDaysToKeepStr: '', artifactNumToKeepStr: '', daysToKeepStr: '30', numToKeepStr: '') // keep builds for 30 days
	}

	agent {
		label 'rust-crossbuild'
	}

	stages {
		stage('Check and Build') {
			steps {
				withCache(name: 'rust-cargo-cache', baseFolder: '$HOME/.cargo', contents: 'registry') {
					sh "cargo check"

					sh "cargo build --release --target=x86_64-unknown-linux-musl"
					sh "cargo build --release --target=x86_64-pc-windows-gnu"
					sh "cargo build --release --target=x86_64-apple-darwin"
				}
			}

			post {
				success {
					archiveArtifacts artifacts: 'target/*/release/codedx-client, target/*/release/codedx-client.exe', fingerprint: true
				}

				failure {
					script {
						slack.error "codedx-cli-client build failed (<${env.BUILD_URL}|Open>)\n[${env.GIT_BRANCH - 'origin/'}: ${env.GIT_COMMIT}]"
					}
				}
			}
		}
	}
}
