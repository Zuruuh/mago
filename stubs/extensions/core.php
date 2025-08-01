<?php

class stdClass
{
}

/**
 * @template TKey
 * @template-covariant TValue
 */
interface Traversable
{
}

/**
 * @template TKey
 * @template-covariant TValue
 * @template-extends Traversable<TKey, TValue>
 */
interface IteratorAggregate extends Traversable
{
    /**
     * @return Traversable<TKey, TValue>
     *
     * @throws Exception
     */
    #[TentativeType]
    public function getIterator(): Traversable;
}

/**
 * @template TKey
 * @template-covariant TValue
 * @template-extends Traversable<TKey, TValue>
 */
interface Iterator extends Traversable
{
    /**
     * @return TValue
     */
    #[TentativeType]
    public function current(): mixed;

    #[TentativeType]
    public function next(): void;

    /**
     * @return TKey|null
     */
    #[TentativeType]
    public function key(): mixed;

    #[TentativeType]
    public function valid(): bool;

    #[TentativeType]
    public function rewind(): void;
}

/**
 * @template TKey
 * @template TValue
 */
interface ArrayAccess
{
    /**
     * @param TKey $offset
     *
     * @no-named-arguments
     */
    #[TentativeType]
    public function offsetExists(mixed $offset): bool;

    /**
     * @param TKey $offset
     *
     * @return TValue
     *
     * @no-named-arguments
     */
    #[TentativeType]
    public function offsetGet(mixed $offset): mixed;

    /**
     * @param TKey $offset
     * @param TValue $value
     *
     * @no-named-arguments
     */
    #[TentativeType]
    public function offsetSet(mixed $offset, mixed $value): void;

    /**
     * @param TKey $offset
     *
     * @no-named-arguments
     */
    #[TentativeType]
    public function offsetUnset(mixed $offset): void;
}

interface Serializable
{
    /**
     * @return string|null
     *
     * @throws Exception
     */
    public function serialize();

    /**
     * @param string $data
     * @return void
     */
    public function unserialize(string $data);
}

interface Throwable extends Stringable
{
    public function getMessage(): string;

    /**
     * @return int|string
     */
    public function getCode();

    public function getFile(): string;

    public function getLine(): int;

    public function getTrace(): array;

    public function getTraceAsString(): string;

    public function getPrevious(): Throwable|null;

    /**
     * @return string
     */
    public function __toString();
}

class Exception implements Throwable
{
    protected $message;

    protected $code;

    protected string $file;

    protected int $line;

    /**
     * @pure
     */
    public function __construct(string $message = '', int $code = 0, null|Throwable $previous = null) {}

    /**
     * @mutation-free
     */
    final public function getMessage(): string
    {
    }

    /**
     * @return int|string
     *
     * @mutation-free
     */
    final public function getCode()
    {
    }

    /**
     * @mutation-free
     */
    final public function getFile(): string
    {
    }

    /**
     * @mutation-free
     */
    final public function getLine(): int
    {
    }

    /**
     * @mutation-free
     */
    final public function getTrace(): array
    {
    }

    /**
     * @mutation-free
     */
    final public function getPrevious(): null|Throwable
    {
    }

    /**
     * @mutation-free
     */
    final public function getTraceAsString(): string
    {
    }

    #[TentativeType]
    public function __toString(): string
    {
    }

    private function __clone(): void
    {
    }

    #[TentativeType]
    public function __wakeup(): void
    {
    }
}

class Error implements Throwable
{
    protected $message;

    protected $code;

    protected string $file;

    protected int $line;

    /**
     * @param string $message
     * @param int $code
     * @param null|Throwable $previous
     *
     * @pure
     */
    public function __construct(string $message = '', int $code = 0, null|Throwable $previous = null) {}

    /**
     * @return string
     *
     * @mutation-free
     */
    final public function getMessage(): string
    {
    }

    /**
     * @return int
     *
     * @mutation-free
     */
    final public function getCode()
    {
    }

    /**
     * @mutation-free
     */
    final public function getFile(): string
    {
    }

    /**
     * @mutation-free
     */
    final public function getLine(): int
    {
    }

    /**
     * @mutation-free
     */
    final public function getTrace(): array
    {
    }

    /**
     * @mutation-free
     */
    final public function getTraceAsString(): string
    {
    }

    /**
     * @mutation-free
     */
    final public function getPrevious(): null|Throwable
    {
    }

    public function __toString(): string
    {
    }

    private function __clone(): void
    {
    }

    #[TentativeType]
    public function __wakeup(): void
    {
    }
}

class ValueError extends Error
{
}

class TypeError extends Error
{
}

class ParseError extends CompileError
{
}

class ArgumentCountError extends TypeError
{
}

class ArithmeticError extends Error
{
}

class CompileError extends Error
{
}

class DivisionByZeroError extends ArithmeticError
{
}

class UnhandledMatchError extends Error
{
}

class RequestParseBodyException extends Exception
{
}

class ErrorException extends Exception
{
    protected int $severity;

    /**
     * @pure
     */
    public function __construct(
        string $message = '',
        int $code = 0,
        int $severity = 1,
        null|string $filename = null,
        null|int $line = null,
        null|Throwable $previous = null,
    ) {}

    /**
     * @mutation-free
     */
    final public function getSeverity(): int
    {
    }
}

final class Closure
{
    private function __construct() {}

    /**
     * @no-named-arguments
     */
    public function __invoke(...$_)
    {
    }

    /**
     * @param object|null $newThis
     * @param object|class-string|null $newScope
     *
     * @return Closure|null
     *
     * @pure
     * @no-named-arguments
     */
    public function bindTo(null|object $newThis, object|string|null $newScope = 'static'): null|Closure
    {
    }

    /**
     * @param Closure $closure
     * @param object|null $newThis
     * @param object|class-string|null $newScope
     *
     * @return Closure|null
     *
     * @pure
     * @no-named-arguments
     */
    public static function bind(
        Closure $closure,
        null|object $newThis,
        object|string|null $newScope = 'static',
    ): null|Closure {
    }

    /**
     * @param object $newThis
     */
    public function call(object $newThis, mixed ...$args): mixed
    {
    }

    public static function fromCallable(callable $callback): Closure
    {
    }
}

interface Countable
{
    #[TentativeType]
    public function count(): int;
}

/**
 * @template T of object
 */
final class WeakReference
{
    public function __construct() {}

    /**
     * @template Input of object
     *
     * @param Input $object
     *
     * @return WeakReference<Input>
     *
     * @pure
     */
    public static function create(object $object): WeakReference
    {
    }

    /**
     * @return T|null
     *
     * @pure
     */
    public function get(): null|object
    {
    }
}

/**
 * @template TKey of object
 * @template TValue
 *
 * @template-implements IteratorAggregate<TKey, TValue>
 */
final class WeakMap implements ArrayAccess, Countable, IteratorAggregate
{
    /**
     * @param TKey $object
     *
     * @pure
     * @no-named-arguments
     */
    public function offsetExists($object): bool
    {
    }

    /**
     * @param TKey $object
     *
     * @return TValue
     *
     * @pure
     * @no-named-arguments
     */
    public function offsetGet($object): mixed
    {
    }

    /**
     * @param TKey $object
     * @param TValue $value
     *
     * @no-named-arguments
     */
    public function offsetSet($object, mixed $value): void
    {
    }

    /**
     * @param TKey $object
     *
     * @no-named-arguments
     */
    public function offsetUnset($object): void
    {
    }

    /**
     * @return Iterator<TKey, TValue>
     *
     * @pure
     */
    public function getIterator(): Iterator
    {
    }

    /**
     * @return int<0,max>
     *
     * @pure
     */
    public function count(): int
    {
    }
}

interface Stringable
{
    public function __toString(): string;
}

#[Attribute(Attribute::TARGET_CLASS)]
final class Attribute
{
    public const TARGET_CLASS = 1;
    public const TARGET_FUNCTION = 2;
    public const TARGET_METHOD = 4;
    public const TARGET_PROPERTY = 8;
    public const TARGET_CLASS_CONSTANT = 16;
    public const TARGET_PARAMETER = 32;
    public const TARGET_ALL = 63;
    public const IS_REPEATABLE = 64;

    public int $flags;

    public function __construct(int $flags = self::TARGET_ALL) {}
}

final class InternalIterator implements Iterator
{
    private function __construct() {}

    public function current(): mixed
    {
    }

    public function next(): void
    {
    }

    public function key(): mixed
    {
    }

    public function valid(): bool
    {
    }

    public function rewind(): void
    {
    }
}

interface UnitEnum
{
    public readonly string $name;

    /**
     * @return list<static>
     *
     * @pure
     */
    public static function cases(): array;
}

interface BackedEnum extends UnitEnum
{
    public readonly int|string $value;

    /**
     * @throws ValueError
     * @throws TypeError
     *
     * @pure
     */
    public static function from(int|string $value): static;

    /**
     * @pure
     */
    public static function tryFrom(int|string $value): null|static;
}

/**
 * @internal
 */
interface IntBackedEnum extends BackedEnum
{
    public readonly int $value;

    /**
     * @throws ValueError
     *
     * @pure
     */
    public static function from(int $value): static;

    /**
     * @pure
     */
    public static function tryFrom(int $value): null|static;
}

/**
 * @internal
 */
interface StringBackedEnum extends BackedEnum
{
    public readonly string $value;

    /**
     * @throws ValueError
     *
     * @pure
     */
    public static function from(string $value): static;

    /**
     * @pure
     */
    public static function tryFrom(string $value): null|static;
}

/**
 * @template TStart
 * @template TResume
 * @template TReturn
 * @template TSuspend
 */
final class Fiber
{
    public function __construct(callable $callback) {}

    /**
     * @param TStart ...$args
     *
     * @return TSuspend|null
     *
     * @throws FiberError
     * @throws Throwable
     */
    public function start(mixed ...$args): mixed
    {
    }

    /**
     * @param TResume $value
     *
     * @return TSuspend|null
     *
     * @throws FiberError
     * @throws Throwable
     */
    public function resume(mixed $value = null): mixed
    {
    }

    /**
     * @return TSuspend|null
     *
     * @throws FiberError
     * @throws Throwable
     */
    public function throw(Throwable $exception): mixed
    {
    }

    public function isStarted(): bool
    {
    }

    public function isSuspended(): bool
    {
    }

    public function isRunning(): bool
    {
    }

    public function isTerminated(): bool
    {
    }

    /**
     * @return TReturn
     *
     * @throws FiberError
     */
    public function getReturn(): mixed
    {
    }

    public static function getCurrent(): null|Fiber
    {
    }

    /**
     * @param TSuspend $value
     *
     * @return TResume
     *
     * @throws FiberError
     * @throws Throwable
     */
    public static function suspend(mixed $value = null): mixed
    {
    }
}

final class FiberError extends Error
{
    public function __construct() {}
}

#[Attribute(Attribute::TARGET_METHOD)]
final class ReturnTypeWillChange
{
    public function __construct() {}
}

#[Attribute(Attribute::TARGET_CLASS)]
final class AllowDynamicProperties
{
    public function __construct() {}
}

#[Attribute(Attribute::TARGET_PARAMETER)]
final class SensitiveParameter
{
    public function __construct() {}
}

final class SensitiveParameterValue
{
    private readonly mixed $value;

    public function __construct(mixed $value) {}

    public function getValue(): mixed
    {
    }

    public function __debugInfo(): array
    {
    }
}

#[Attribute(Attribute::TARGET_METHOD)]
final class Override
{
    public function __construct() {}
}

#[Attribute(Attribute::TARGET_METHOD | Attribute::TARGET_FUNCTION | Attribute::TARGET_CLASS_CONSTANT)]
final class Deprecated
{
    public readonly null|string $message;
    public readonly null|string $since;

    public function __construct(null|string $message = null, null|string $since = null) {}
}

/**
 * @return non-empty-string
 *
 * @pure
 * @no-named-arguments
 */
function zend_version(): string
{
}

/**
 * @pure
 * @no-named-arguments
 */
function func_num_args(): int
{
}

/**
 * @pure
 * @no-named-arguments
 */
function func_get_arg(int $position): mixed
{
}

/**
 * @pure
 * @no-named-arguments
 */
function func_get_args(): array
{
}

/**
 * @return int<0, max>
 *
 * @pure
 * @no-named-arguments
 */
function strlen(string $string): int
{
}

/**
 * @pure
 * @no-named-arguments
 */
function strcmp(string $string1, string $string2): int
{
}

/**
 * @pure
 * @no-named-arguments
 */
function strncmp(string $string1, string $string2, int $length): int
{
}

/**
 * @pure
 * @no-named-arguments
 */
function strcasecmp(string $string1, string $string2): int
{
}

/**
 * @pure
 * @no-named-arguments
 */
function strncasecmp(string $string1, string $string2, int $length): int
{
}

/**
 * @pure
 * @no-named-arguments
 */
function str_starts_with(string $haystack, string $needle): bool
{
}

/**
 * @pure
 * @no-named-arguments
 */
function str_ends_with(string $haystack, string $needle): bool
{
}

/**
 * @pure
 * @no-named-arguments
 */
function str_contains(string $haystack, string $needle): bool
{
}

/**
 * @pure
 * @no-named-arguments
 */
function str_decrement(string $string): string
{
}

/**
 * @pure
 * @no-named-arguments
 */
function str_increment(string $string): string
{
}

/**
 * @no-named-arguments
 */
function error_reporting(null|int $error_level): int
{
}

/**
 * @no-named-arguments
 */
function define(string $constant_name, mixed $value, bool $case_insensitive = false): bool
{
}

/**
 * @pure
 * @no-named-arguments
 */
function defined(string $constant_name): bool
{
}

/**
 * @pure
 * @no-named-arguments
 */
function get_class(object $object): string
{
}

/**
 * @pure
 * @no-named-arguments
 */
function get_called_class(): string
{
}

/**
 * @pure
 * @no-named-arguments
 */
function get_parent_class(object|string $object_or_class): string|false
{
}

/**
 * @param object|class-string $object_or_class
 *
 * @pure
 * @no-named-arguments
 */
function method_exists(object|string $object_or_class, string $method): bool
{
}

/**
 * @param object|class-string $object_or_class
 *
 * @pure
 * @no-named-arguments
 */
function property_exists(object|string $object_or_class, string $property): bool
{
}

/**
 * @param string $trait
 *
 * @assert-if-true trait-string $trait
 *
 * @pure
 * @no-named-arguments
 */
function trait_exists(string $trait, bool $autoload = true): bool
{
}

/**
 * @param string $class
 *
 * @assert-if-true class-string $class
 *
 * @pure
 * @no-named-arguments
 */
function class_exists(string $class, bool $autoload = true): bool
{
}

/**
 * @param string $interface
 *
 * @assert-if-true interface-string $interface
 *
 * @pure
 * @no-named-arguments
 */
function interface_exists(string $interface, bool $autoload = true): bool
{
}

/**
 * @param string $function
 *
 * @assert-if-true non-empty-string $function
 *
 * @pure
 * @no-named-arguments
 */
function function_exists(string $function): bool
{
}

/**
 * @param string $enum
 *
 * @assert-if-true enum-string $enum
 *
 * @pure
 * @no-named-arguments
 */
function enum_exists(string $enum, bool $autoload = true): bool
{
}

/**
 * @param class-string $class
 * @param non-empty-string $alias
 *
 * @pure
 * @no-named-arguments
 */
function class_alias(string $class, string $alias, bool $autoload = true): bool
{
}

/**
 * @return list<non-empty-string>
 *
 * @pure
 */
function get_included_files(): array
{
}

/**
 * @return list<non-empty-string>
 *
 * @pure
 */
function get_required_files(): array
{
}

/**
 * @param object|class-string $object_or_class
 * @param class-string $class
 *
 * @pure
 */
function is_subclass_of(mixed $object_or_class, string $class, bool $allow_string = true): bool
{
}

/**
 * @param object|class-string $object_or_class
 * @param class-string $class
 *
 * @pure
 */
function is_a(mixed $object_or_class, string $class, bool $allow_string = false): bool
{
}

/**
 * @param class-string $class
 *
 * @pure
 */
function get_class_vars(string $class): array
{
}

/**
 * @param object $object
 *
 * @pure
 */
function get_object_vars(object $object): array
{
}

/**
 * @param object|class-string $object_or_class
 * @return list<non-empty-string>
 *
 * @pure
 */
function get_class_methods(object|string $object_or_class): array
{
}

function trigger_error(string $message, int $error_level = E_USER_NOTICE)
{
}

function user_error(string $message, int $error_level = E_USER_NOTICE)
{
}

/**
 * @return callable|null
 */
function set_error_handler(null|callable $callback, int $error_levels = E_ALL)
{
}

function restore_error_handler(): true
{
}

/**
 * @return callable|null
 */
function set_exception_handler(null|callable $callback)
{
}

function restore_exception_handler(): true
{
}

/**
 * @return list<class-string>
 *
 * @pure
 */
function get_declared_classes(): array
{
}

/**
 * @return list<interface-string>
 *
 * @pure
 */
function get_declared_interfaces(): array
{
}

/**
 * @return list<trait-string>
 *
 * @pure
 */
function get_declared_traits(): array
{
}

/**
 * @return array{internal: list<non-empty-string>, user: list<non-empty-string>}
 *
 * @pure
 */
function get_defined_functions(bool $exclude_disabled = true): array
{
}

/**
 * @pure
 */
function get_defined_vars(): array
{
}

/**
 * @param resource $resource
 *
 * @return non-empty-string
 *
 * @pure
 */
function get_resource_type($resource): string
{
}

/**
 * @return non-empty-list<non-empty-string>
 *
 * @pure
 */
function get_loaded_extensions(bool $zend_extensions = false): array
{
}

/**
 * @param non-empty-string $extension
 *
 * @pure
 */
function extension_loaded(string $extension): bool
{
}

/**
 * @return list<non-empty-string>|false
 *
 * @pure
 */
function get_extension_funcs(string $extension): array|false
{
}

/**
 * @pure
 */
function get_defined_constants(bool $categorize = false): array
{
}

/**
 * @pure
 */
function debug_backtrace(int $options = DEBUG_BACKTRACE_PROVIDE_OBJECT, int $limit = 0): array
{
}

function debug_print_backtrace(int $options = 0, #[PhpStormStubsElementAvailable(from: '7.0')] int $limit = 0): void
{
}

function gc_collect_cycles(): int
{
}

/**
 * @pure
 */
function gc_enabled(): bool
{
}

function gc_enable(): void
{
}

function gc_disable(): void
{
}

/**
 * @return array{runs: int, collected: int, threshold: int, roots: int}
 *
 * @pure
 */
function gc_status(): array
{
}

function gc_mem_caches(): int
{
}

/**
 * @return list<resource>
 *
 * @pure
 */
function get_resources(null|string $type): array
{
}

function exit(string|int $status = 0): never
{
}

function die(string|int $status = 0): never
{
}

/**
 * @deprecated
 */
const E_STRICT = 2048;

const E_ERROR = 1;

const E_RECOVERABLE_ERROR = 4096;

const E_WARNING = 2;

const E_PARSE = 4;

const E_NOTICE = 8;

const E_DEPRECATED = 8192;

const E_CORE_ERROR = 16;

const E_CORE_WARNING = 32;

const E_COMPILE_ERROR = 64;

const E_COMPILE_WARNING = 128;

const E_USER_ERROR = 256;

const E_USER_WARNING = 512;

const E_USER_NOTICE = 1024;

const E_USER_DEPRECATED = 16384;

const E_ALL = 30719;

const DEBUG_BACKTRACE_PROVIDE_OBJECT = 1;

const DEBUG_BACKTRACE_IGNORE_ARGS = 2;

const S_MEMORY = 1;

const S_VARS = 4;

const S_FILES = 8;

const S_INCLUDE = 16;

const S_SQL = 32;

const S_EXECUTOR = 64;

const S_MAIL = 128;

const S_SESSION = 256;

const S_MISC = 2;

const S_INTERNAL = 536870912;

const S_ALL = 511;

const ZEND_THREAD_SAFE = false;

const ZEND_DEBUG_BUILD = false;

const PHP_WINDOWS_VERSION_BUILD = 0;

const PHP_WINDOWS_VERSION_MAJOR = 0;

const PHP_WINDOWS_VERSION_MINOR = 0;

const PHP_WINDOWS_VERSION_PLATFORM = 0;

const PHP_WINDOWS_VERSION_PRODUCTTYPE = 0;

const PHP_WINDOWS_VERSION_SP_MAJOR = 0;

const PHP_WINDOWS_VERSION_SP_MINOR = 0;

const PHP_WINDOWS_VERSION_SUITEMASK = 0;

const PHP_WINDOWS_NT_DOMAIN_CONTROLLER = 2;

const PHP_WINDOWS_NT_SERVER = 3;

const PHP_WINDOWS_NT_WORKSTATION = 1;

const PHP_WINDOWS_EVENT_CTRL_C = 0;

const PHP_WINDOWS_EVENT_CTRL_BREAK = 1;

const PHP_VERSION = PHP_VERSION;

const PHP_MAJOR_VERSION = PHP_MAJOR_VERSION;

const PHP_MINOR_VERSION = PHP_MINOR_VERSION;

const PHP_RELEASE_VERSION = PHP_RELEASE_VERSION;

const PHP_EXTRA_VERSION = PHP_EXTRA_VERSION;

const PHP_VERSION_ID = PHP_VERSION_ID;

const PHP_ZTS = PHP_ZTS;

const PHP_DEBUG = PHP_DEBUG;

const PHP_OS = PHP_OS;

const PHP_OS_FAMILY = PHP_OS_FAMILY;

const PHP_SAPI = PHP_SAPI;

const PHP_CLI_PROCESS_TITLE = PHP_CLI_PROCESS_TITLE;

const DEFAULT_INCLUDE_PATH = DEFAULT_INCLUDE_PATH;

const PEAR_INSTALL_DIR = PEAR_INSTALL_DIR;

const PEAR_EXTENSION_DIR = PEAR_EXTENSION_DIR;

const PHP_EXTENSION_DIR = PHP_EXTENSION_DIR;

const PHP_BINARY = PHP_BINARY;

const PHP_PREFIX = PHP_PREFIX;

const PHP_BINDIR = PHP_BINDIR;

const PHP_LIBDIR = PHP_LIBDIR;

const PHP_DATADIR = PHP_DATADIR;

const PHP_SYSCONFDIR = PHP_SYSCONFDIR;

const PHP_LOCALSTATEDIR = PHP_LOCALSTATEDIR;

const PHP_CONFIG_FILE_PATH = PHP_CONFIG_FILE_PATH;

const PHP_CONFIG_FILE_SCAN_DIR = PHP_CONFIG_FILE_SCAN_DIR;

const PHP_SHLIB_SUFFIX = PHP_SHLIB_SUFFIX;

const PHP_EOL = PHP_EOL;

const SUHOSIN_PATCH = 1;

const SUHOSIN_PATCH_VERSION = SUHOSIN_PATCH_VERSION;

const PHP_MAXPATHLEN = 4096;

const PHP_INT_MAX = 9223372036854775807;

const PHP_INT_MIN = -9223372036854775808;

const PHP_INT_SIZE = 8;

const PHP_FLOAT_DIG = 15;

const PHP_FLOAT_EPSILON = 2.2204460492503e-16;

const PHP_FLOAT_MAX = 1.7976931348623e+308;

const PHP_FLOAT_MIN = 2.2250738585072e-308;

const ZEND_MULTIBYTE = 0;

const PHP_OUTPUT_HANDLER_START = 1;

const PHP_OUTPUT_HANDLER_CONT = 2;

const PHP_OUTPUT_HANDLER_END = 4;

const UPLOAD_ERR_OK = 0;

const UPLOAD_ERR_INI_SIZE = 1;

const UPLOAD_ERR_FORM_SIZE = 2;

const UPLOAD_ERR_PARTIAL = 3;

const UPLOAD_ERR_NO_FILE = 4;

const UPLOAD_ERR_NO_TMP_DIR = 6;

const UPLOAD_ERR_CANT_WRITE = 7;

const UPLOAD_ERR_EXTENSION = 8;

const STDIN = fopen('php://stdin', 'r');

const STDOUT = fopen('php://stdout', 'w');

const STDERR = fopen('php://stderr', 'w');

const PHP_FD_SETSIZE = 1024;

const PHP_OUTPUT_HANDLER_WRITE = 0;

const PHP_OUTPUT_HANDLER_FLUSH = 4;

const PHP_OUTPUT_HANDLER_CLEAN = 2;

const PHP_OUTPUT_HANDLER_FINAL = 8;

const PHP_OUTPUT_HANDLER_CLEANABLE = 16;

const PHP_OUTPUT_HANDLER_FLUSHABLE = 32;

const PHP_OUTPUT_HANDLER_REMOVABLE = 64;

const PHP_OUTPUT_HANDLER_STDFLAGS = 112;

const PHP_OUTPUT_HANDLER_STARTED = 4096;

const PHP_OUTPUT_HANDLER_DISABLED = 8192;

const PHP_MANDIR = '/usr/local/php/php/man';

const PHP_SBINDIR = '/usr/local/sbin', PHP_OUTPUT_HANDLER_PROCESSED = 16384;
