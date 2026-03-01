package Yggdrasil::Engine;

use strict;
use warnings;

our $VERSION = '0.1.0';

use File::Basename qw(dirname);
use File::Spec;
use FFI::Platypus 2.00;
use JSON::PP qw(decode_json encode_json);

use Yggdrasil::CustomStrategyHandler;

my $STATUS_OK = 'Ok';
my $STATUS_NOT_FOUND = 'NotFound';
my $STATUS_ERROR = 'Error';

my $ffi = FFI::Platypus->new(api => 2);
$ffi->lib(_get_binary_path());

$ffi->attach([new_engine => '_new_engine'] => [] => 'opaque');
$ffi->attach([free_engine => '_free_engine'] => ['opaque'] => 'void');
$ffi->attach([take_state => '_take_state'] => ['opaque', 'string'] => 'opaque');
$ffi->attach([check_enabled => '_check_enabled'] => ['opaque', 'string', 'string', 'string'] => 'opaque');
$ffi->attach([check_variant => '_check_variant'] => ['opaque', 'string', 'string', 'string'] => 'opaque');
$ffi->attach([should_emit_impression_event => '_should_emit_impression_event'] => ['opaque', 'string'] => 'opaque');
$ffi->attach([free_response => '_free_response'] => ['opaque'] => 'void');
$ffi->attach([get_metrics => '_get_metrics'] => ['opaque'] => 'opaque');

$ffi->attach([count_toggle => '_count_toggle'] => ['opaque', 'string', 'bool'] => 'opaque');
$ffi->attach([count_variant => '_count_variant'] => ['opaque', 'string', 'string'] => 'opaque');

$ffi->attach([define_counter => '_define_counter'] => ['opaque', 'string', 'string'] => 'opaque');
$ffi->attach([inc_counter => '_inc_counter'] => ['opaque', 'string', 'sint64', 'string'] => 'opaque');
$ffi->attach([collect_impact_metrics => '_collect_impact_metrics'] => ['opaque'] => 'opaque');
$ffi->attach([restore_impact_metrics => '_restore_impact_metrics'] => ['opaque', 'string'] => 'opaque');
$ffi->attach([define_gauge => '_define_gauge'] => ['opaque', 'string', 'string'] => 'opaque');
$ffi->attach([set_gauge => '_set_gauge'] => ['opaque', 'string', 'double', 'string'] => 'opaque');
$ffi->attach([define_histogram => '_define_histogram'] => ['opaque', 'string', 'string', 'string'] => 'opaque');
$ffi->attach([observe_histogram => '_observe_histogram'] => ['opaque', 'string', 'double', 'string'] => 'opaque');

sub new {
    my ($class) = @_;

    my $state = _new_engine();

    return bless {
        state                   => $state,
        custom_strategy_handler => Yggdrasil::CustomStrategyHandler->new(),
    }, $class;
}

sub DESTROY {
    my ($self) = @_;

    if ($self->{state}) {
        _free_engine($self->{state});
        $self->{state} = undef;
    }

    return;
}

sub take_state {
    my ($self, $state_json) = @_;

    my $response = $self->_read_response(_take_state($self->{state}, $state_json));
    $self->{custom_strategy_handler}->update_strategies($state_json);

    _throw_if_error($response);
    return $response->{value};
}

sub is_enabled {
    my ($self, $toggle_name, $context) = @_;

    my $context_json = encode_json($context || {});
    my $custom_strategy_results = encode_json(
        $self->{custom_strategy_handler}->evaluate_custom_strategies($toggle_name, $context || {})
    );

    my $response = $self->_read_response(
        _check_enabled($self->{state}, $toggle_name, $context_json, $custom_strategy_results)
    );

    _throw_if_error($response);
    return undef if $response->{status_code} eq $STATUS_NOT_FOUND;

    return $response->{value} ? 1 : 0;
}

sub get_variant {
    my ($self, $toggle_name, $context) = @_;

    my $context_json = encode_json($context || {});
    my $custom_strategy_results = encode_json(
        $self->{custom_strategy_handler}->evaluate_custom_strategies($toggle_name, $context || {})
    );

    my $response = $self->_read_response(
        _check_variant($self->{state}, $toggle_name, $context_json, $custom_strategy_results)
    );

    _throw_if_error($response);
    return undef if $response->{status_code} eq $STATUS_NOT_FOUND;

    return $response->{value};
}

sub register_custom_strategies {
    my ($self, $strategies) = @_;
    $self->{custom_strategy_handler}->register_custom_strategies($strategies);
    return;
}

sub count_toggle {
    my ($self, $toggle_name, $enabled) = @_;
    my $response = $self->_read_response(_count_toggle($self->{state}, $toggle_name, $enabled ? 1 : 0));
    _throw_if_error($response);
    return;
}

sub count_variant {
    my ($self, $toggle_name, $variant_name) = @_;
    my $response = $self->_read_response(_count_variant($self->{state}, $toggle_name, $variant_name));
    _throw_if_error($response);
    return;
}

sub get_metrics {
    my ($self) = @_;

    my $response = $self->_read_response(_get_metrics($self->{state}));
    _throw_if_error($response);

    return $response->{value};
}

sub should_emit_impression_event {
    my ($self, $toggle_name) = @_;

    my $response = $self->_read_response(_should_emit_impression_event($self->{state}, $toggle_name));
    _throw_if_error($response);

    return $response->{value} ? 1 : 0;
}

sub define_counter {
    my ($self, $name, $help_text) = @_;

    my $response = $self->_read_response(_define_counter($self->{state}, $name, $help_text));
    _throw_if_error($response);
    return;
}

sub inc_counter {
    my ($self, $name, $value, $labels) = @_;

    $value = 1 unless defined $value;
    my $labels_json = defined $labels ? encode_json($labels) : undef;

    my $response = $self->_read_response(_inc_counter($self->{state}, $name, $value, $labels_json));
    _throw_if_error($response);
    return;
}

sub collect_impact_metrics {
    my ($self) = @_;

    my $response = $self->_read_response(_collect_impact_metrics($self->{state}));
    _throw_if_error($response);

    return $response->{value} || [];
}

sub restore_impact_metrics {
    my ($self, $metrics) = @_;

    my $metrics_json = encode_json($metrics || []);
    my $response = $self->_read_response(_restore_impact_metrics($self->{state}, $metrics_json));
    _throw_if_error($response);
    return;
}

sub define_gauge {
    my ($self, $name, $help_text) = @_;

    my $response = $self->_read_response(_define_gauge($self->{state}, $name, $help_text));
    _throw_if_error($response);
    return;
}

sub set_gauge {
    my ($self, $name, $value, $labels) = @_;

    my $labels_json = defined $labels ? encode_json($labels) : undef;
    my $response = $self->_read_response(_set_gauge($self->{state}, $name, $value, $labels_json));
    _throw_if_error($response);
    return;
}

sub define_histogram {
    my ($self, $name, $help_text, $buckets) = @_;

    my $buckets_json = encode_json($buckets || []);
    my $response = $self->_read_response(_define_histogram($self->{state}, $name, $help_text, $buckets_json));
    _throw_if_error($response);
    return;
}

sub observe_histogram {
    my ($self, $name, $value, $labels) = @_;

    my $labels_json = defined $labels ? encode_json($labels) : undef;
    my $response = $self->_read_response(_observe_histogram($self->{state}, $name, $value, $labels_json));
    _throw_if_error($response);
    return;
}

sub _read_response {
    my ($self, $response_ptr) = @_;

    die 'native function returned null response pointer' if !$response_ptr;

    my $response_json;
    my $cast_ok = eval {
        $response_json = $ffi->cast('opaque', 'string', $response_ptr);
        1;
    };

    _free_response($response_ptr);

    die $@ if !$cast_ok;

    return decode_json($response_json);
}

sub _throw_if_error {
    my ($response) = @_;

    if (($response->{status_code} || q{}) eq $STATUS_ERROR) {
        my $message = $response->{error_message} || 'Unknown FFI error';
        die "Error: $message";
    }

    return;
}

sub _get_binary_path {
    if ($ENV{YGGDRASIL_LIB_PATH}) {
        my $path = File::Spec->catfile($ENV{YGGDRASIL_LIB_PATH}, _library_name());
        return $path if -f $path;
    }

    my $lib_dir = File::Spec->catdir(dirname(__FILE__), 'lib');
    my $path = File::Spec->catfile($lib_dir, _library_name());

    if (!-f $path) {
        die "Could not find native library at '$path'. Run perl-engine/build.sh first or set YGGDRASIL_LIB_PATH.";
    }

    return $path;
}

sub _library_name {
    return 'libyggdrasilffi.so' if $^O eq 'linux';
    return 'libyggdrasilffi.dylib' if $^O eq 'darwin';
    return 'yggdrasilffi.dll' if $^O =~ /MSWin32|cygwin|msys/;

    die "Unsupported operating system: $^O";
}

1;
