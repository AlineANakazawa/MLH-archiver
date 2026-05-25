#!/usr/bin/env perl
# Create an example public-inbox v2 repository from .eml fixture files
use strict;
use warnings;
use v5.10.1;
use File::Path qw(make_path remove_tree);
use File::Find;
use File::Spec;
use Cwd qw(abs_path);

eval {
    require PublicInbox::TestCommon;
    require PublicInbox::Eml;
    require PublicInbox::InboxWritable;
    require PublicInbox::Import;
    1;
} or do {
    die "Failed to load required modules: $@\n";
};

my $eml_dir = shift @ARGV
    or die "Usage: $0 <eml_directory> <output_directory>\n";
my $out_dir = shift @ARGV
    or die "Usage: $0 <eml_directory> <output_directory>\n";

die "EML directory not found: $eml_dir\n" unless -d $eml_dir;

make_path($out_dir) unless -d $out_dir;

say "Scanning for .eml files in: $eml_dir";

my @eml_files;
find(sub {
    push @eml_files, $File::Find::name if /\.eml$/;
}, $eml_dir);

die "No .eml files found in $eml_dir\n" unless @eml_files;

my @emails;
foreach my $file (sort @eml_files) {
    open(my $fh, '<', $file) or do {
        warn "Cannot open $file: $!\n";
        next;
    };
    local $/;
    my $raw = <$fh>;
    close $fh;
    eval {
        my $eml = PublicInbox::Eml->new($raw);
        push @emails, $eml;
    };
    if ($@) {
        warn "Failed to parse $file: $@\n";
    }
}

say "Parsed " . scalar(@emails) . " emails from " . scalar(@eml_files) . " .eml files";

my $inbox_name = "example";
my $inbox_dir = File::Spec->catdir($out_dir, "v2_$inbox_name");

say "Creating V2 inbox: $inbox_name";

PublicInbox::TestCommon::create_inbox(
    $inbox_name,
    version => 2,
    tmpdir => $inbox_dir,
    sub {
        my ($importer, $ibx) = @_;
        foreach my $eml (@emails) {
            $importer->add($eml);
        }
    }
);

say "Inbox created at: $inbox_dir";

my $t_dir = File::Spec->catdir($out_dir, 't');
if (-d $t_dir) {
    File::Path::remove_tree($t_dir);
    say "Cleaned up temp directory: $t_dir";
}

unlink glob("$inbox_dir/creat.*");
unlink "$inbox_dir/inbox.lock";
unlink "$inbox_dir/open.lock";
unlink "$inbox_dir/msgmap.sqlite3-journal";
