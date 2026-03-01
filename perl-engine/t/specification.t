use strict;
use warnings;

use Test::More;
use JSON::PP qw(decode_json);
use File::Spec;
use FindBin;

BEGIN {
    eval { require FFI::Platypus; 1 } or plan skip_all => 'FFI::Platypus is required for client specification tests';
}

use lib File::Spec->catdir($FindBin::Bin, '..', 'lib');
use Yggdrasil::Engine;

if ($ENV{SKIP_CLIENT_SPEC}) {
    plan skip_all => 'set SKIP_CLIENT_SPEC=0 (or unset) to run client specification tests';
}

my $spec_dir = File::Spec->catdir($FindBin::Bin, '..', '..', 'client-specification', 'specifications');
my $index_file = File::Spec->catfile($spec_dir, 'index.json');

if (!-f $index_file) {
    plan skip_all => 'client-specification/specifications/index.json not found';
}

sub _load_json_file {
    my ($path) = @_;
    open my $fh, '<', $path or die "cannot read $path: $!";
    my $json = do { local $/; <$fh> };
    close $fh;
    return decode_json($json);
}

sub _normalize_scalar {
    my ($value) = @_;

    if (ref($value) =~ /Boolean$/) {
        return $value ? 1 : 0;
    }

    return $value;
}

sub _normalize_variant {
    my ($variant) = @_;

    return _normalize_scalar($variant) if ref($variant) ne 'HASH';

    my %copy = %{$variant};

    # Canonicalize key naming across bindings/specs.
    if (exists $copy{feature_enabled} && !exists $copy{featureEnabled}) {
        $copy{featureEnabled} = delete $copy{feature_enabled};
    }

    if (exists $copy{payload} && !defined $copy{payload}) {
        delete $copy{payload};
    }

    for my $k (keys %copy) {
        if (ref($copy{$k}) eq 'HASH') {
            my %nested = %{ _normalize_variant($copy{$k}) };
            $copy{$k} = \%nested;
        } elsif (ref($copy{$k}) eq 'ARRAY') {
            my @arr = map { ref($_) eq 'HASH' ? _normalize_variant($_) : _normalize_scalar($_) } @{ $copy{$k} };
            $copy{$k} = \@arr;
        } else {
            $copy{$k} = _normalize_scalar($copy{$k});
        }
    }

    return \%copy;
}

sub _default_disabled_variant {
    return {
        name => 'disabled',
        enabled => 0,
        featureEnabled => 0,
    };
}

sub _safe_label {
    my ($label) = @_;
    $label = '' if !defined $label;
    $label =~ s/[^\x20-\x7E]/?/g;
    return $label;
}

my $spec_index = _load_json_file($index_file);
my $engine = Yggdrasil::Engine->new();

for my $spec_file (@{$spec_index}) {
    my $spec_data = _load_json_file(File::Spec->catfile($spec_dir, $spec_file));
    my $spec_name = $spec_data->{name} || $spec_file;
    my $state = $spec_data->{state} || {};
    my $tests = $spec_data->{tests} || [];
    my $variant_tests = $spec_data->{variantTests} || [];

    $engine->take_state(JSON::PP::encode_json($state));

    for my $test (@{$tests}) {
        my $toggle_name = $test->{toggleName};
        my $context = $test->{context};
        my $expected = _normalize_scalar($test->{expectedResult}) ? 1 : 0;

        my $actual = $engine->is_enabled($toggle_name, $context);
        $actual = defined $actual ? ($actual ? 1 : 0) : 0;

        is($actual, $expected, _safe_label("$spec_name: $test->{description}"));
    }

    for my $test (@{$variant_tests}) {
        my $toggle_name = $test->{toggleName};
        my $context = $test->{context};

        my $expected = _normalize_variant($test->{expectedResult});
        my $actual_raw = $engine->get_variant($toggle_name, $context);
        $actual_raw = _default_disabled_variant() if !defined $actual_raw;
        my $actual = _normalize_variant($actual_raw);

        is_deeply($actual, $expected, _safe_label("$spec_name: $test->{description}"));
    }
}

done_testing();
