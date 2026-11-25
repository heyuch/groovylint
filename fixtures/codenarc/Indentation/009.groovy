
abstract class AbstractRuleFactory implements RuleFactory {
    @Override
    protected abstract Rule createRule()

    protected abstract Rule createErrorRule()
}
        