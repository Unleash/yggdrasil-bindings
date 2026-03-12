package Yggdrasil::CustomStrategyHandler;

use strict;
use warnings;

use JSON::PP qw(decode_json);

my %STANDARD_STRATEGIES = map { $_ => 1 } (
    'default',
    'userWithId',
    'gradualRolloutUserId',
    'gradualRolloutSessionId',
    'gradualRolloutRandom',
    'flexibleRollout',
    'remoteAddress',
);

sub new {
    my ($class) = @_;

    return bless {
        strategy_definitions     => {},
        strategy_implementations => {},
    }, $class;
}

sub update_strategies {
    my ($self, $features_json) = @_;

    my $parsed = decode_json($features_json);
    my $features = _get_features_json($parsed);

    my %custom_strategies;
    for my $toggle (@{$features}) {
        my @toggle_strategies = grep {
            !$STANDARD_STRATEGIES{$_->{name}}
        } @{ $toggle->{strategies} || [] };

        if (@toggle_strategies) {
            $custom_strategies{$toggle->{name}} = \@toggle_strategies;
        }
    }

    $self->{strategy_definitions} = \%custom_strategies;
    return;
}

sub register_custom_strategies {
    my ($self, $custom_strategies) = @_;

    if (ref($custom_strategies) ne 'HASH') {
        die 'custom_strategies must be a hash reference';
    }

    while (my ($name, $strategy) = each %{$custom_strategies}) {
        if (ref($strategy) eq 'CODE') {
            $self->{strategy_implementations}{$name} = $strategy;
            next;
        }

        if (ref($strategy) && $strategy->can('apply')) {
            $self->{strategy_implementations}{$name} = $strategy;
            next;
        }

        die "Custom strategy '$name' must be a code reference or an object with apply";
    }

    return;
}

sub evaluate_custom_strategies {
    my ($self, $toggle_name, $context) = @_;

    my %results;
    my $strategies = $self->{strategy_definitions}{$toggle_name} || [];

    for (my $i = 0; $i < @{$strategies}; $i++) {
        my $strategy = $strategies->[$i];
        my $strategy_impl = $self->{strategy_implementations}{ $strategy->{name} };

        my $key = sprintf('customStrategy%d', $i + 1);
        $results{$key} = _apply_strategy($strategy_impl, $strategy->{parameters} || {}, $context || {});
    }

    return \%results;
}

sub _apply_strategy {
    my ($strategy_impl, $parameters, $context) = @_;

    return JSON::PP::false unless $strategy_impl;

    if (ref($strategy_impl) eq 'CODE') {
        return $strategy_impl->($parameters, $context) ? JSON::PP::true : JSON::PP::false;
    }

    return $strategy_impl->apply($parameters, $context) ? JSON::PP::true : JSON::PP::false;
}

sub _get_features_json {
    my ($message) = @_;

    if (ref($message) eq 'HASH' && exists $message->{features}) {
        return $message->{features};
    }

    if (ref($message) eq 'HASH' && exists $message->{events}) {
        my %features;

        for my $event (@{ $message->{events} || [] }) {
            if ($event->{type} eq 'feature-updated') {
                my $feature = $event->{feature};
                $features{ $feature->{name} } = $feature if $feature;
            } elsif ($event->{type} eq 'feature-removed') {
                delete $features{ $event->{featureName} };
            } elsif ($event->{type} eq 'hydration') {
                %features = map { $_->{name} => $_ } @{ $event->{features} || [] };
            }
        }

        return [values %features];
    }

    return [];
}

1;
