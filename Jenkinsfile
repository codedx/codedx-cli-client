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
				script {
					github.setBuildStatus('codedx/codedx-cli-client', env.GIT_COMMIT, 'package/jenkins/build.codedx.io', 'pending', 'Preparing to build...')
				}

				withCache(name: 'rust-cargo-cache', baseFolder: '$HOME/.cargo', contents: 'registry') {
					script {
						github.setBuildStatus('codedx/codedx-cli-client', env.GIT_COMMIT, 'package/jenkins/build.codedx.io', 'pending', '`cargo check`')
					}

					sh "cargo check"

					script {
						github.setBuildStatus('codedx/codedx-cli-client', env.GIT_COMMIT, 'package/jenkins/build.codedx.io', 'pending', '`cargo build`')
					}

					sh "cargo build --release --target=x86_64-unknown-linux-musl"
					sh "cargo build --release --target=x86_64-pc-windows-gnu"
					sh "cargo build --release --target=x86_64-apple-darwin"
				}
			}

			post {
				success {
					script {
						github.setBuildStatus('codedx/codedx-cli-client', env.GIT_COMMIT, 'package/jenkins/build.codedx.io', 'success', '')
					}

					archiveArtifacts artifacts: 'target/*/release/codedx-client, target/*/release/codedx-client.exe', fingerprint: true
				}

				failure {
					script {
						github.setBuildStatus('codedx/codedx-cli-client', env.GIT_COMMIT, 'package/jenkins/build.codedx.io', 'failed', 'Build failed')
					}
				}
			}
		}
	}
}
