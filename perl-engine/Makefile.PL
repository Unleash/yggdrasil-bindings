use strict;
use warnings;

use ExtUtils::MakeMaker;

WriteMakefile(
    NAME         => 'Yggdrasil::Engine',
    VERSION_FROM => 'lib/Yggdrasil/Engine.pm',
    ABSTRACT     => 'Perl FFI bindings for the Unleash Yggdrasil engine',
    AUTHOR       => 'Unleash',
    LICENSE      => 'apache_2_0',
    PREREQ_PM    => {
        'FFI::Platypus' => '2.00',
        'JSON::PP'      => '0',
        'Test::More'    => '0',
    },
    MIN_PERL_VERSION => '5.20.0',
);
