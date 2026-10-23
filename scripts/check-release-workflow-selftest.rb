#!/usr/bin/env ruby

require 'fileutils'
require 'open3'
require 'tmpdir'
require 'yaml'

workflow = YAML.load_file(ARGV.fetch(0, File.expand_path('../.github/workflows/release.yml', __dir__)))
verify = workflow.fetch('jobs').fetch('verify').fetch('steps')
publish = workflow.fetch('jobs').fetch('publish').fetch('steps')

def step(steps, name)
  steps.find { |entry| entry['name'] == name }.fetch('run')
end

def check(condition, message)
  abort message unless condition
end

def git(dir, *args)
  stdout, stderr, status = Open3.capture3('git', '-C', dir, *args)
  check(status.success?, "fixture git failed: #{stderr}")
  stdout.strip
end

def run_step(script, dir, env, expected, output = {})
  path = File.join(dir, 'step-output')
  File.write(path, '')
  stdout, stderr, status = Open3.capture3(env.merge('GITHUB_OUTPUT' => path), 'bash', '-c', script, chdir: dir)
  check(status.success? == expected, "unexpected step result: #{stdout}#{stderr}")
  check(File.read(path).lines.map(&:strip).sort == output.map { |key, value| "#{key}=#{value}" }.sort,
        'unexpected workflow outputs')
end

Dir.mktmpdir('release-workflow-selftest-') do |dir|
  git(dir, 'init', '-q')
  git(dir, 'config', 'user.name', 'release-fixture')
  git(dir, 'config', 'user.email', '322824348+mb2090@users.noreply.github.com')
  git(dir, 'config', 'commit.gpgsign', 'false')
  git(dir, 'config', 'tag.gpgsign', 'false')
  File.write(File.join(dir, 'Cargo.toml'), "[workspace.package]\nversion = \"0.1.3\"\n")
  git(dir, 'add', 'Cargo.toml')
  git(dir, 'commit', '-qm', 'Release fixture')
  commit = git(dir, 'rev-parse', 'HEAD')
  git(dir, 'update-ref', 'refs/remotes/origin/main', commit)
  git(dir, 'tag', '-a', 'v0.1.3', '-m', "Fixture\n\n-----BEGIN SSH SIGNATURE-----\nfixture\n-----END SSH SIGNATURE-----")
  object = git(dir, 'rev-parse', 'refs/tags/v0.1.3')
  env = { 'TAG' => 'v0.1.3', 'GITHUB_EVENT_NAME' => 'workflow_dispatch', 'GITHUB_REF' => 'refs/heads/main',
          'EXPECTED_COMMIT' => '' }

  select = step(verify, 'Select release tag')
  run_step(select, dir, env, true, 'tag' => 'v0.1.3')
  ['', 'main', 'v01.1.3', 'v0.1.3/other', 'v0.1.3;touch injected'].each do |invalid|
    run_step(select, dir, env.merge('TAG' => invalid), false)
  end
  check(!File.exist?(File.join(dir, 'injected')), 'tag input executed a shell command')
  run_step(select, dir, env.merge('GITHUB_REF' => 'refs/heads/other'), false)
  run_step(select, dir, env.merge('GITHUB_EVENT_NAME' => 'push', 'GITHUB_REF' => 'refs/tags/v0.1.3'), true,
           'tag' => 'v0.1.3')

  tag_check = step(verify, 'Check the tag')
  expected = { 'commit' => commit, 'tag_object' => object }
  run_step(tag_check, dir, env, true, expected)
  run_step(tag_check, dir, env.merge('EXPECTED_COMMIT' => commit), true, expected)
  run_step(tag_check, dir, env.merge('EXPECTED_COMMIT' => '0' * 40), false)
  run_step(tag_check, dir, env.merge('TAG' => 'v0.1.4'), false)
  git(dir, 'tag', 'v0.1.4')
  run_step(tag_check, dir, env.merge('TAG' => 'v0.1.4'), false)
  git(dir, 'tag', '-a', '-f', 'v0.1.3', '-m', 'Unsigned fixture')
  run_step(tag_check, dir, env, false)
  git(dir, 'update-ref', 'refs/tags/v0.1.3', object)
  File.write(File.join(dir, 'Cargo.toml'), "[workspace.package]\nversion = \"0.1.4\"\n")
  run_step(tag_check, dir, env, false)
  File.write(File.join(dir, 'Cargo.toml'), "[workspace.package]\nversion = \"0x1x3\"\n")
  run_step(tag_check, dir, env, false)
  git(dir, 'checkout', '--', 'Cargo.toml')
  git(dir, 'commit', '--allow-empty', '-qm', 'Not released')
  newer = git(dir, 'rev-parse', 'HEAD')
  run_step(tag_check, dir, env, false)
  git(dir, 'update-ref', 'refs/remotes/origin/main', newer)
  git(dir, 'checkout', '-q', '--detach', commit)
  run_step(tag_check, dir, env, true, expected)
  git(dir, 'update-ref', 'refs/remotes/origin/main', newer)
  git(dir, 'checkout', '-q', '--orphan', 'unrelated')
  git(dir, 'commit', '--allow-empty', '-qm', 'Unrelated main')
  git(dir, 'update-ref', 'refs/remotes/origin/main', 'HEAD')
  git(dir, 'checkout', '-q', '--detach', commit)
  run_step(tag_check, dir, env, false)

  checkout = step(publish, 'Check publication checkout')
  publication_env = env.merge('RELEASE_COMMIT' => commit, 'TAG_OBJECT' => object)
  run_step(checkout, dir, publication_env, true)
  run_step(checkout, dir, publication_env.merge('RELEASE_COMMIT' => newer), false)
  run_step(checkout, dir, publication_env.merge('TAG_OBJECT' => commit), false)

  bin = File.join(dir, 'bin')
  FileUtils.mkdir_p(bin)
  fake_gh = File.join(bin, 'gh')
  File.write(fake_gh, <<~'SH')
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ $1 == api && $2 == graphql ]]; then
      if [[ -n ${FIXTURE_STATE:-} && -f $FIXTURE_STATE ]]; then
        cat "$FIXTURE_STATE"
      else
        printf '%s' "${FIXTURE_RELEASE_ID:-}"
      fi
      exit "${FIXTURE_API_EXIT:-0}"
    fi
    [[ $1 == release && $2 == create && $3 == v0.1.3 ]]
    [[ ! -f $FIXTURE_STATE ]]
    echo created > "$FIXTURE_STATE"
    echo create >> "$FIXTURE_CALLS"
  SH
  FileUtils.chmod(0o700, fake_gh)
  api_env = { 'PATH' => "#{bin}:#{ENV.fetch('PATH')}", 'TAG' => 'v0.1.3', 'GITHUB_REPOSITORY' => 'example/project' }
  existing = step(publish, 'Check for existing release')
  run_step(existing, dir, api_env, true, 'exists' => 'false')
  2.times { run_step(existing, dir, api_env.merge('FIXTURE_RELEASE_ID' => 'existing'), true, 'exists' => 'true') }
  run_step(existing, dir, api_env.merge('FIXTURE_API_EXIT' => '1'), false)
  state = File.join(dir, 'release-state')
  calls = File.join(dir, 'release-calls')
  retry_env = api_env.merge('FIXTURE_STATE' => state, 'FIXTURE_CALLS' => calls)
  run_step(existing, dir, retry_env, true, 'exists' => 'false')
  FileUtils.mkdir_p(File.join(dir, 'dist'))
  File.write(File.join(dir, 'dist', 'asset.tar.gz'), 'fixture')
  run_step(step(publish, 'Create GitHub release'), dir, retry_env, true)
  2.times { run_step(existing, dir, retry_env, true, 'exists' => 'true') }
  check(File.read(calls) == "create\n", 'publication retries created a duplicate release')

  File.write(File.join(dir, 'Cargo.toml'), "[workspace.package]\nversion = \"9.9.9\"\n")
  git(dir, 'add', 'Cargo.toml')
  git(dir, 'commit', '-qm', 'Newer workflow revision')
  binary = File.join(dir, 'target', 'release', 'examples')
  FileUtils.mkdir_p(binary)
  File.write(File.join(binary, 'hello_box'), 'fixture binary')
  archive_env = env.merge('RELEASE_COMMIT' => commit, 'GITHUB_SHA' => git(dir, 'rev-parse', 'HEAD'))
  run_step(step(publish, 'Package assets'), dir, archive_env, true)
  source, stderr, status = Open3.capture3('tar', '-xOf', File.join(dir, 'dist', 'lunatui-v0.1.3-source.tar.gz'),
                                       'lunatui-0.1.3/Cargo.toml')
  check(status.success?, stderr)
  check(source == "[workspace.package]\nversion = \"0.1.3\"\n", 'source archive used the workflow revision instead of the tag')
end

publish.drop(1).each do |entry|
  check(entry['if'] == "steps.existing.outputs.exists != 'true'", 'publication step does not skip existing release')
end
check(workflow.fetch('concurrency').fetch('cancel-in-progress') == false, 'release concurrency cancels active publication')
check(step(publish, 'Package assets').include?('"$RELEASE_COMMIT"'), 'source archive is not pinned to verified commit')
puts 'check-release-workflow-selftest: ok'
