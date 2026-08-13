#!/usr/bin/env ruby
# Extract ecosystem facts from a dependabot-core checkout, using Ruby's own
# parser rather than a regex over method bodies.
#
# Dependabot states these as code — `filenames.include?("Cargo.toml")` — and a
# regex would have to guess at string boundaries, interpolation and comments.
# Ripper is stdlib, is the parser Ruby itself uses, and gets it exactly right.
#
#   ruby tools/extract-dependabot.rb <checkout> > facts.json

require "ripper"
require "json"

# Every string literal beneath a node, in source order.
def strings(node, out = [])
  return out unless node.is_a?(Array)
  if node[0] == :@tstring_content && node[1].is_a?(String)
    out << node[1]
    return out
  end
  node.each { |child| strings(child, out) if child.is_a?(Array) }
  out
end

# The body of a singleton method (`def self.name`), or nil.
def singleton_method(node, name, found = [])
  return found unless node.is_a?(Array)
  if node[0] == :defs
    ident = node[3]
    found << node if ident.is_a?(Array) && ident[1] == name
  end
  node.each { |child| singleton_method(child, name, found) if child.is_a?(Array) }
  found
end

# `CONST = "value"` or `CONST = %w(a b)` at any depth. Values are kept as
# lists, because the ecosystems that matter here name several files.
def constants(node, out = {})
  return out unless node.is_a?(Array)
  if node[0] == :assign && node[1].is_a?(Array) && node[1][0] == :var_field
    target = node[1][1]
    if target.is_a?(Array) && target[0] == :@const
      values = strings(node[2])
      out[target[1]] = values unless values.empty?
    end
  end
  node.each { |child| constants(child, out) if child.is_a?(Array) }
  out
end

# Constant names referenced beneath a node, bare or qualified.
def referenced(node, out = [])
  return out unless node.is_a?(Array)
  out << node[1] if node[0] == :@const && node[1].is_a?(String)
  node.each { |child| referenced(child, out) if child.is_a?(Array) }
  out
end

def parse(path)
  return nil unless File.exist?(path)
  Ripper.sexp(File.read(path))
end

root = ARGV[0] or abort "usage: extract-dependabot.rb <checkout>"
out = []
Dir.glob(File.join(root, "*/lib/dependabot/*/file_fetcher.rb")).sort.each do |fetcher|
  dir = File.dirname(fetcher)
  slug = File.basename(dir)
  tree = parse(fetcher)
  next unless tree

  method = singleton_method(tree, "required_files_in?")
  required = method.flat_map { |node| strings(node) }
  message = singleton_method(tree, "required_files_message").flat_map { |node| strings(node) }

  # Every constant this ecosystem defines, from every file it defines them in.
  # `required_files_in?` frequently names one rather than spelling the filename
  # out — composer says PackageManager::MANIFEST_FILENAME, deno says
  # MANIFEST_FILENAMES — so the reference has to be followed to be read.
  consts = constants(tree)
  Dir.glob(File.join(dir, "**", "*.rb")).sort.each do |sibling|
    sub = parse(sibling)
    constants(sub).each { |name, values| consts[name] ||= values }
  end

  method.flat_map { |node| referenced(node) }.uniq.each do |name|
    required.concat(consts[name]) if consts[name]
  end

  out << {
    "slug" => slug,
    "ecosystem" => consts["ECOSYSTEM"],
    "package_manager" => consts["PACKAGE_MANAGER"],
    "required_files" => required.uniq,
    "lockfiles" => consts.select { |name, _| name.match?(/LOCK/) }.values.flatten.uniq,
    "resolved_via_constant" => !singleton_method(tree, "required_files_in?")
      .flat_map { |node| referenced(node) }.empty?,
    "message" => message.join(" ").strip,
  }
end
puts JSON.pretty_generate(out)
