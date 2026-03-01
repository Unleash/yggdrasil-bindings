use strict;
use warnings;

use Test::More;
use File::Spec;

BEGIN {
    eval { require FFI::Platypus; 1 } or plan skip_all => 'FFI::Platypus is required for perl-engine tests';
}

use lib File::Spec->catdir(File::Spec->curdir(), 'lib');
use Yggdrasil::Engine;

my $custom_strategy_state = <<'JSON';
{
  "version": 1,
  "features": [
    {
      "name": "Feature.A",
      "enabled": true,
      "strategies": [
        {
          "name": "breadStrategy",
          "parameters": {}
        }
      ],
      "variants": [
        {
          "name": "sourDough",
          "weight": 100
        }
      ],
      "impressionData": true
    }
  ]
}
JSON

my $engine = Yggdrasil::Engine->new();

$engine->register_custom_strategies({
    breadStrategy => sub {
        my ($parameters, $context) = @_;
        return $context->{betterThanSlicedBread} ? 1 : 0;
    },
});

$engine->take_state($custom_strategy_state);

is($engine->is_enabled('Feature.A', { betterThanSlicedBread => 1 }), 1, 'is_enabled true for matching custom strategy');
is($engine->is_enabled('Feature.A', { betterThanSlicedBread => 0 }), 0, 'is_enabled false for non-matching custom strategy');

my $variant = $engine->get_variant('Feature.A', { betterThanSlicedBread => 1 });
is($variant->{name}, 'sourDough', 'get_variant returns expected variant');

$engine->count_toggle('some-toggle', 1);
my $metrics = $engine->get_metrics();
is($metrics->{toggles}->{'some-toggle'}->{yes}, 1, 'count_toggle and get_metrics work');

$engine->define_counter('test_counter', 'Test counter');
$engine->inc_counter('test_counter', 2);
$engine->inc_counter('test_counter', 3, { env => 'test' });

my $impact = $engine->collect_impact_metrics();
ok(ref($impact) eq 'ARRAY', 'collect_impact_metrics returns an array');
ok(scalar(@{$impact}) >= 1, 'impact metrics has entries after updates');

$engine->restore_impact_metrics($impact);
pass('restore_impact_metrics does not raise');

done_testing();
