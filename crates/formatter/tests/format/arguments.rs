use indoc::indoc;

use mago_formatter::settings::FormatSettings;

use crate::test_format;

#[test]
pub fn test_expand_last_argument() {
    let code = indoc! {r#"
        <?php

        $value = strtr($this->fileExcerpt($trace['file'], $trace['line'], 5), [
            '#DD0000' => 'var(--highlight-string)',
            '#007700' => 'var(--highlight-keyword)',
            '#0000BB' => 'var(--highlight-default)',
            '#FF8000' => 'var(--highlight-comment)',
        ]);
    "#};

    let expected = indoc! {r#"
        <?php

        $value = strtr($this->fileExcerpt($trace['file'], $trace['line'], 5), [
            '#DD0000' => 'var(--highlight-string)',
            '#007700' => 'var(--highlight-keyword)',
            '#0000BB' => 'var(--highlight-default)',
            '#FF8000' => 'var(--highlight-comment)',
        ]);
    "#};

    test_format(code, expected, FormatSettings::default());
}

#[test]
pub fn test_expand_first_argument() {
    let code = indoc! {r#"
        <?php

        return $propertyMetadata->withSchema(($this->addNullabilityToTypeDefinition)([
            'type' => 'string',
            'format' => 'decimal',
        ], $type));
    "#};

    let expected = indoc! {r#"
        <?php

        return $propertyMetadata->withSchema(($this->addNullabilityToTypeDefinition)([
            'type' => 'string',
            'format' => 'decimal',
        ], $type));
    "#};

    test_format(code, expected, FormatSettings::default());
}

#[test]
pub fn test_hug_new() {
    let code = indoc! {r#"
        <?php

        function something(): void {
            $result = App::call(new DeployMosquito(new WebhookData(artifactUuid: $uuid, deploymentTarget: $target, service: Service::ERP)));
        }
    "#};

    let expected = indoc! {r#"
        <?php

        function something(): void
        {
            $result = App::call(new DeployMosquito(new WebhookData(
                artifactUuid: $uuid,
                deploymentTarget: $target,
                service: Service::ERP,
            )));
        }
    "#};

    test_format(code, expected, FormatSettings::default());
}

#[test]
pub fn test_hug_new_with_few_simple_args() {
    let code = indoc! {r#"
        <?php

        function something(): void {
          return Vec\values(
              new FilesystemIterator($directory, FilesystemIterator::CURRENT_AS_PATHNAME | FilesystemIterator::SKIP_DOTS),
          );
        }
    "#};

    let expected = indoc! {r#"
        <?php

        function something(): void
        {
            return Vec\values(new FilesystemIterator(
                $directory,
                FilesystemIterator::CURRENT_AS_PATHNAME | FilesystemIterator::SKIP_DOTS,
            ));
        }
    "#};

    test_format(code, expected, FormatSettings::default());
}

#[test]
pub fn test_hug_last_new_with_named_args() {
    let code = indoc! {r#"
        <?php

        $foo = (new \Lcobucci\JWT\Validation\Validator())->assert($token, new SignedWith(
            signer: new Sha256(),
            key: InMemory::plainText($this->jwtKey),
        ));
    "#};

    let expected = indoc! {r#"
        <?php

        $foo = new \Lcobucci\JWT\Validation\Validator()->assert($token, new SignedWith(
            signer: new Sha256(),
            key: InMemory::plainText($this->jwtKey),
        ));
    "#};

    test_format(code, expected, FormatSettings::default());
}
