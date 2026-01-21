require 'rspec'
require 'json'
require_relative '../lib/yggdrasil_engine'

RSpec.describe YggdrasilEngine do
  let(:yggdrasil_engine) { YggdrasilEngine.new }

  describe '#inc_counter' do
    it 'should increment counter value' do
      yggdrasil_engine.define_counter('test_counter', 'Test counter')
      yggdrasil_engine.inc_counter('test_counter', 5)
      yggdrasil_engine.inc_counter('test_counter', 3)

      metrics = yggdrasil_engine.collect_impact_metrics()
      counter = metrics.find { |m| m[:name] == 'test_counter' }

      expect(counter).not_to be_nil
      expect(counter[:help]).to eq('Test counter')
      expect(counter[:samples].length).to eq(1)
      expect(counter[:samples][0][:value]).to eq(8)
    end

    it 'should increment counter with labels' do
      yggdrasil_engine.define_counter('test_counter', 'Test counter')
      yggdrasil_engine.inc_counter('test_counter', 5, { 'env' => 'test' })
      yggdrasil_engine.inc_counter('test_counter', 3, { 'env' => 'prod' })

      metrics = yggdrasil_engine.collect_impact_metrics()
      counter = metrics.find { |m| m[:name] == 'test_counter' }

      expect(counter).not_to be_nil
      expect(counter[:samples].length).to eq(2)

      test_sample = counter[:samples].find { |s| s[:labels][:env] == 'test' }
      prod_sample = counter[:samples].find { |s| s[:labels][:env] == 'prod' }

      expect(test_sample[:value]).to eq(5)
      expect(prod_sample[:value]).to eq(3)
    end
  end

  describe '#set_gauge' do
    it 'should set gauge value' do
      yggdrasil_engine.define_gauge('test_gauge', 'Test gauge')
      yggdrasil_engine.set_gauge('test_gauge', 5)
      yggdrasil_engine.set_gauge('test_gauge', 10)

      metrics = yggdrasil_engine.collect_impact_metrics()
      gauge = metrics.find { |m| m[:name] == 'test_gauge' }

      expect(gauge).not_to be_nil
      expect(gauge[:help]).to eq('Test gauge')
      expect(gauge[:samples].length).to eq(1)
      expect(gauge[:samples][0][:value]).to eq(10)
    end

    it 'should set gauge with labels' do
      yggdrasil_engine.define_gauge('test_gauge', 'Test gauge')
      yggdrasil_engine.set_gauge('test_gauge', 5, { 'env' => 'test' })
      yggdrasil_engine.set_gauge('test_gauge', 3, { 'env' => 'prod' })

      metrics = yggdrasil_engine.collect_impact_metrics()
      gauge = metrics.find { |m| m[:name] == 'test_gauge' }

      expect(gauge).not_to be_nil
      expect(gauge[:samples].length).to eq(2)

      test_sample = gauge[:samples].find { |s| s[:labels][:env] == 'test' }
      prod_sample = gauge[:samples].find { |s| s[:labels][:env] == 'prod' }

      expect(test_sample[:value]).to eq(5)
      expect(prod_sample[:value]).to eq(3)
    end
  end

  describe '#observe_histogram' do
    it 'should observe histogram values' do
      yggdrasil_engine.define_histogram('request_duration', 'Request duration', [0.1, 0.5, 1.0, 5.0])
      yggdrasil_engine.observe_histogram('request_duration', 0.05)
      yggdrasil_engine.observe_histogram('request_duration', 0.75)
      yggdrasil_engine.observe_histogram('request_duration', 3.0)

      metrics = yggdrasil_engine.collect_impact_metrics()
      histogram = metrics.find { |m| m[:name] == 'request_duration' }

      expect(histogram).not_to be_nil
      expect(histogram[:help]).to eq('Request duration')
      expect(histogram[:type]).to eq('histogram')
      expect(histogram[:samples].length).to eq(1)
    end

    it 'should observe histogram with labels' do
      yggdrasil_engine.define_histogram('request_duration', 'Request duration', [0.1, 0.5, 1.0, 5.0])
      yggdrasil_engine.observe_histogram('request_duration', 0.05, { 'env' => 'test' })
      yggdrasil_engine.observe_histogram('request_duration', 0.75, { 'env' => 'prod' })

      metrics = yggdrasil_engine.collect_impact_metrics()
      histogram = metrics.find { |m| m[:name] == 'request_duration' }

      expect(histogram).not_to be_nil
      expect(histogram[:samples].length).to eq(2)

      test_sample = histogram[:samples].find { |s| s[:labels][:env] == 'test' }
      prod_sample = histogram[:samples].find { |s| s[:labels][:env] == 'prod' }

      expect(test_sample).not_to be_nil
      expect(prod_sample).not_to be_nil
    end
  end

  describe '#define_histogram' do
    it 'should define histogram with default buckets' do
      yggdrasil_engine.define_histogram('request_duration', 'Request duration')
      yggdrasil_engine.observe_histogram('request_duration', 0.05)

      metrics = yggdrasil_engine.collect_impact_metrics()
      histogram = metrics.find { |m| m[:name] == 'request_duration' }

      expect(histogram).not_to be_nil
      expect(histogram[:type]).to eq('histogram')
      expect(histogram[:samples].length).to eq(1)
    end
  end

  describe '#collect_impact_metrics' do
    it 'should return empty list when no metrics' do
      metrics = yggdrasil_engine.collect_impact_metrics()
      expect(metrics).to eq([])
    end
  end

  describe '#restore_impact_metrics' do
    it 'should restore impact metrics' do
      yggdrasil_engine.define_counter('test_counter', 'Test counter')
      yggdrasil_engine.inc_counter('test_counter', 10)
      yggdrasil_engine.define_gauge('test_gauge', 'Test gauge')
      yggdrasil_engine.set_gauge('test_gauge', 42)
      yggdrasil_engine.define_histogram('test_histogram', 'Test histogram', [0.1, 0.5, 1.0])
      yggdrasil_engine.observe_histogram('test_histogram', 0.25)

      metrics = yggdrasil_engine.collect_impact_metrics()
      expect(metrics.length).to eq(3)

      counter = metrics.find { |m| m[:name] == 'test_counter' }
      gauge = metrics.find { |m| m[:name] == 'test_gauge' }
      histogram = metrics.find { |m| m[:name] == 'test_histogram' }

      expect(counter[:samples][0][:value]).to eq(10)
      expect(gauge[:samples][0][:value]).to eq(42)
      expect(histogram).not_to be_nil

      yggdrasil_engine.restore_impact_metrics(metrics)

      restored_metrics = yggdrasil_engine.collect_impact_metrics()
      expect(restored_metrics.length).to eq(3)

      restored_counter = restored_metrics.find { |m| m[:name] == 'test_counter' }
      restored_gauge = restored_metrics.find { |m| m[:name] == 'test_gauge' }
      restored_histogram = restored_metrics.find { |m| m[:name] == 'test_histogram' }

      expect(restored_counter[:samples][0][:value]).to eq(10)
      expect(restored_gauge[:samples][0][:value]).to eq(42)
      expect(restored_histogram).not_to be_nil
    end
  end
end
